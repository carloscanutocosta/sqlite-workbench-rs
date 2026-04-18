# SQLite Workbench

> **[Português](README.pt-PT.md)** · **English** · **[Français](README.fr.md)**

A desktop GUI application for browsing and editing SQLite databases, built with Rust and [egui](https://github.com/emilk/egui).

## Features

- **Open databases** — `.db`, `.sqlite`, `.sqlite3` files
- **Data tab** — browse, filter, sort and paginate table rows; inline edit, insert and delete records
- **Schema tab** — view the SQL CREATE statement for each table
- **Stats tab** — per-column statistics (row count, nulls, unique values, min/max/avg, top values); export to JSON
- **SQL editor** — execute arbitrary queries, keyword and table/column autocomplete, Ctrl+Enter shortcut, query history and favourites
- **CSV import** — load a CSV file directly into a new table
- **CSV / JSON export** — export the current view or SQL results
- **Create table** — visual dialog with column definitions and foreign key support
- **Rename / drop tables** — via right-click context menu
- **ERD view** — entity-relationship diagram showing tables and foreign keys
- **Vacuum** — compact and optimise the database file
- **Dark / Light theme** — switchable at runtime
- **Multilingual UI** — Portuguese and English

## Requirements

- [Rust](https://rustup.rs/) 1.75 or later

## Build

```bash
cargo build --release
```

The binary is produced at `target/release/sqlite-workbench-rs`.

## Run

```bash
cargo run --release
```

Or execute the compiled binary directly and open a database from **File → Open database**.

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Execute SQL query |

## Settings

Settings (language, theme, rows per page) are persisted in `settings.json` next to the binary. Query history is stored in `history.json` and favourites in `favorites.json`.

## License

Licensed under the [European Union Public Licence v1.2](LICENSE) (EUPL-1.2).
