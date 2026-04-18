use rusqlite::{Connection, Result as SqlResult, params};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: String,
    pub pk: bool,
    pub ai: bool,
    pub nn: bool,
}

#[derive(Debug, Clone)]
pub struct ForeignKeyDef {
    pub from_col: String,
    pub ref_table: String,
    pub ref_col: String,
}

#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub from_col: String,
    pub ref_table: String,
    pub ref_col: String,
}

#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    pub column: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct ColumnStats {
    pub total_rows: i64,
    pub non_null_count: i64,
    pub null_count: i64,
    pub unique_count: i64,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub avg_value: Option<f64>,
    pub top_values: Vec<(String, i64)>,
}

pub struct DatabaseManager {
    pub conn: Connection,
    pub path: String,
}

impl DatabaseManager {
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(Path::new(path))
            .map_err(|e| format!("Failed to open database: {e}"))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("Failed to set pragmas: {e}"))?;
        Ok(Self { conn, path: path.to_string() })
    }

    pub fn get_tables(&self) -> Result<Vec<String>, String> {
        let mut stmt = self.conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;")
            .map_err(|e| e.to_string())?;
        let tables = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tables)
    }

    pub fn get_columns(&self, table: &str) -> Result<Vec<String>, String> {
        let mut stmt = self.conn
            .prepare(&format!("PRAGMA table_info('{}')", table.replace('\'', "''")))
            .map_err(|e| e.to_string())?;
        let cols = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(cols)
    }

    pub fn get_column_types(&self, table: &str) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self.conn
            .prepare(&format!("PRAGMA table_info('{}')", table.replace('\'', "''")))
            .map_err(|e| e.to_string())?;
        let pairs = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(pairs)
    }

    pub fn get_foreign_keys(&self, table: &str) -> Result<Vec<ForeignKey>, String> {
        let mut stmt = self.conn
            .prepare(&format!("PRAGMA foreign_key_list('{}');", table.replace('\'', "''")))
            .map_err(|e| e.to_string())?;
        let fks = stmt
            .query_map([], |row| {
                Ok(ForeignKey {
                    from_col: row.get::<_, String>(3)?,
                    ref_table: row.get::<_, String>(2)?,
                    ref_col: row.get::<_, String>(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(fks)
    }

    fn escape(name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn build_where(&self, table: &str, filter: &FilterConfig) -> (String, Vec<String>) {
        if filter.value.is_empty() {
            return (String::new(), vec![]);
        }
        let valid_ops = ["=", "!=", ">", "<", ">=", "<=", "LIKE"];
        let op = if valid_ops.contains(&filter.operator.as_str()) {
            &filter.operator
        } else {
            "LIKE"
        };

        if filter.column == "All" || filter.column == "Todos" || filter.column.is_empty() {
            if let Ok(cols) = self.get_columns(table) {
                let wildcard = format!("%{}%", filter.value);
                let conditions: Vec<String> = cols.iter()
                    .map(|c| format!("{} LIKE ?", Self::escape(c)))
                    .collect();
                let params = vec![wildcard; cols.len()];
                return (format!(" WHERE {}", conditions.join(" OR ")), params);
            }
            return (String::new(), vec![]);
        }

        let col = Self::escape(&filter.column);
        let val = if op == "LIKE" {
            format!("%{}%", filter.value)
        } else {
            filter.value.clone()
        };
        (format!(" WHERE {col} {op} ?"), vec![val])
    }

    pub fn get_row_count(&self, table: &str, filter: &FilterConfig) -> Result<i64, String> {
        let safe_table = Self::escape(table);
        let (where_clause, params) = self.build_where(table, filter);
        let query = format!("SELECT COUNT(*) FROM {safe_table}{where_clause}");
        let mut stmt = self.conn.prepare(&query).map_err(|e| e.to_string())?;
        let count = stmt
            .query_row(rusqlite::params_from_iter(params.iter()), |row| row.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        Ok(count)
    }

    pub fn get_table_data(
        &self,
        table: &str,
        limit: i64,
        offset: i64,
        filter: &FilterConfig,
        sort_col: Option<&str>,
        sort_asc: bool,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
        let safe_table = Self::escape(table);
        let (where_clause, mut params) = self.build_where(table, filter);

        let order_clause = sort_col
            .map(|c| format!(" ORDER BY {} {}", Self::escape(c), if sort_asc { "ASC" } else { "DESC" }))
            .unwrap_or_default();

        let query = format!("SELECT rowid, * FROM {safe_table}{where_clause}{order_clause} LIMIT ? OFFSET ?");
        params.push(limit.to_string());
        params.push(offset.to_string());

        let mut stmt = self.conn.prepare(&query).map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows: Vec<Vec<String>> = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let mut vals = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let v: String = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => String::from("NULL"),
                        Ok(rusqlite::types::ValueRef::Integer(n)) => n.to_string(),
                        Ok(rusqlite::types::ValueRef::Real(f)) => f.to_string(),
                        Ok(rusqlite::types::ValueRef::Text(t)) => String::from_utf8_lossy(t).into_owned(),
                        Ok(rusqlite::types::ValueRef::Blob(_)) => String::from("[BLOB]"),
                        Err(_) => String::new(),
                    };
                    vals.push(v);
                }
                Ok(vals)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok((col_names, rows))
    }

    pub fn get_table_schema(&self, table: &str) -> String {
        let result: SqlResult<Option<String>> = self.conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?;",
            params![table],
            |row| row.get(0),
        );
        match result {
            Ok(Some(s)) => s,
            _ => String::from("-- Schema not available."),
        }
    }

    pub fn insert_record(&self, table: &str, data: &[(String, String)]) -> Result<(), String> {
        if data.is_empty() {
            return Err("No data provided.".into());
        }
        self.validate_data(table, data)?;
        let safe_table = Self::escape(table);
        let cols: Vec<String> = data.iter().map(|(c, _)| Self::escape(c)).collect();
        let placeholders: Vec<&str> = vec!["?"; data.len()];
        let vals: Vec<&str> = data.iter().map(|(_, v)| v.as_str()).collect();
        let query = format!(
            "INSERT INTO {safe_table} ({}) VALUES ({})",
            cols.join(", "),
            placeholders.join(", ")
        );
        self.conn
            .execute(&query, rusqlite::params_from_iter(vals.iter()))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_record(&self, table: &str, rowid: i64, data: &[(String, String)]) -> Result<(), String> {
        if data.is_empty() {
            return Err("No data provided.".into());
        }
        self.validate_data(table, data)?;
        let safe_table = Self::escape(table);
        let set_parts: Vec<String> = data.iter().map(|(c, _)| format!("{} = ?", Self::escape(c))).collect();
        let mut vals: Vec<String> = data.iter().map(|(_, v)| v.clone()).collect();
        vals.push(rowid.to_string());
        let query = format!("UPDATE {safe_table} SET {} WHERE rowid = ?", set_parts.join(", "));
        let affected = self.conn
            .execute(&query, rusqlite::params_from_iter(vals.iter()))
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err("No record was updated. rowid may be invalid.".into());
        }
        Ok(())
    }

    pub fn delete_record(&self, table: &str, rowid: i64) -> Result<(), String> {
        let query = format!("DELETE FROM {} WHERE rowid = ?", Self::escape(table));
        let affected = self.conn
            .execute(&query, params![rowid])
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err("No record was deleted. rowid may be invalid.".into());
        }
        Ok(())
    }

    pub fn create_table(&self, name: &str, cols: &[ColumnDef], fks: &[ForeignKeyDef]) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Table name cannot be empty.".into());
        }
        if cols.is_empty() {
            return Err("Table must have at least one column.".into());
        }
        let safe_name = Self::escape(name);
        let mut defs: Vec<String> = cols.iter().map(|c| {
            let mut def = format!("{} {}", Self::escape(&c.name), c.col_type);
            if c.pk { def.push_str(" PRIMARY KEY"); }
            if c.ai { def.push_str(" AUTOINCREMENT"); }
            if c.nn { def.push_str(" NOT NULL"); }
            def
        }).collect();
        for fk in fks {
            defs.push(format!(
                "FOREIGN KEY ({}) REFERENCES {} ({})",
                Self::escape(&fk.from_col),
                Self::escape(&fk.ref_table),
                Self::escape(&fk.ref_col),
            ));
        }
        let query = format!("CREATE TABLE {safe_name} ({})", defs.join(", "));
        self.conn.execute(&query, []).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename_table(&self, old: &str, new: &str) -> Result<(), String> {
        let query = format!("ALTER TABLE {} RENAME TO {}", Self::escape(old), Self::escape(new));
        self.conn.execute(&query, []).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn drop_table(&self, name: &str) -> Result<(), String> {
        let query = format!("DROP TABLE {}", Self::escape(name));
        self.conn.execute(&query, []).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn vacuum(&self) -> Result<(), String> {
        self.conn.execute_batch("VACUUM;").map_err(|e| e.to_string())
    }

    pub fn execute_query(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
        let mut stmt = self.conn.prepare(query).map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        if col_names.is_empty() {
            drop(stmt);
            self.conn.execute(query, []).map_err(|e| e.to_string())?;
            return Ok((vec![], vec![]));
        }
        let rows: Vec<Vec<String>> = stmt
            .query_map([], |row| {
                let mut vals = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let v = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => String::from("NULL"),
                        Ok(rusqlite::types::ValueRef::Integer(n)) => n.to_string(),
                        Ok(rusqlite::types::ValueRef::Real(f)) => f.to_string(),
                        Ok(rusqlite::types::ValueRef::Text(t)) => String::from_utf8_lossy(t).into_owned(),
                        Ok(rusqlite::types::ValueRef::Blob(_)) => String::from("[BLOB]"),
                        Err(_) => String::new(),
                    };
                    vals.push(v);
                }
                Ok(vals)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok((col_names, rows))
    }

    pub fn import_csv(&self, csv_path: &str, table_name: &str) -> Result<usize, String> {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(csv_path)
            .map_err(|e| e.to_string())?;

        let headers: Vec<String> = rdr.headers()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();

        if headers.is_empty() {
            return Err("CSV file is empty or has no headers.".into());
        }

        let safe_table = Self::escape(table_name);
        let col_defs: Vec<String> = headers.iter().map(|h| format!("{} TEXT", Self::escape(h))).collect();
        let create_q = format!("CREATE TABLE {safe_table} ({})", col_defs.join(", "));
        self.conn.execute(&create_q, []).map_err(|e| e.to_string())?;

        let escaped_cols: Vec<String> = headers.iter().map(|h| Self::escape(h)).collect();
        let placeholders = vec!["?"; headers.len()].join(", ");
        let insert_q = format!("INSERT INTO {safe_table} ({}) VALUES ({placeholders})", escaped_cols.join(", "));

        let mut count = 0;
        for result in rdr.records() {
            let record = result.map_err(|e| e.to_string())?;
            let vals: Vec<&str> = record.iter().collect();
            self.conn.execute(&insert_q, rusqlite::params_from_iter(vals.iter()))
                .map_err(|e| e.to_string())?;
            count += 1;
        }
        Ok(count)
    }

    pub fn get_column_stats(&self, table: &str, col: &str) -> Result<ColumnStats, String> {
        let safe_table = Self::escape(table);
        let safe_col = Self::escape(col);

        let agg_query = format!(
            "SELECT COUNT(*), COUNT({safe_col}), COUNT(*) - COUNT({safe_col}), \
             COUNT(DISTINCT {safe_col}), MIN({safe_col}), MAX({safe_col}), \
             AVG(CASE WHEN typeof({safe_col}) IN ('integer','real') THEN {safe_col} ELSE NULL END) \
             FROM {safe_table}"
        );

        let mut stats = ColumnStats::default();
        self.conn.query_row(&agg_query, [], |row| {
            stats.total_rows = row.get::<_, i64>(0).unwrap_or(0);
            stats.non_null_count = row.get::<_, i64>(1).unwrap_or(0);
            stats.null_count = row.get::<_, i64>(2).unwrap_or(0);
            stats.unique_count = row.get::<_, i64>(3).unwrap_or(0);
            stats.min_value = row.get::<_, Option<String>>(4).unwrap_or(None);
            stats.max_value = row.get::<_, Option<String>>(5).unwrap_or(None);
            stats.avg_value = row.get::<_, Option<f64>>(6).unwrap_or(None);
            Ok(())
        }).map_err(|e| e.to_string())?;

        let top_query = format!(
            "SELECT {safe_col}, COUNT(*) as freq FROM {safe_table} \
             WHERE {safe_col} IS NOT NULL GROUP BY {safe_col} ORDER BY freq DESC LIMIT 10"
        );
        let mut stmt = self.conn.prepare(&top_query).map_err(|e| e.to_string())?;
        stats.top_values = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, i64>(1).unwrap_or(0),
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(stats)
    }

    fn validate_data(&self, table: &str, data: &[(String, String)]) -> Result<(), String> {
        let types = self.get_column_types(table).unwrap_or_default();
        for (col, val) in data {
            if val.is_empty() { continue; }
            if let Some((_, dtype)) = types.iter().find(|(c, _)| c == col) {
                let up = dtype.to_uppercase();
                if up.contains("INT") {
                    val.parse::<i64>().map_err(|_| {
                        format!("Column '{col}' ({dtype}) expects an integer, got '{val}'.")
                    })?;
                } else if ["REAL", "FLOAT", "DOUBLE", "DECIMAL", "NUMERIC"].iter().any(|t| up.contains(t)) {
                    val.parse::<f64>().map_err(|_| {
                        format!("Column '{col}' ({dtype}) expects a number, got '{val}'.")
                    })?;
                }
            }
        }
        Ok(())
    }
}
