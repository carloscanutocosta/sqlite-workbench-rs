# SQLite Workbench

> **Português** · **[English](README.md)** · **[Français](README.fr.md)**

Aplicação de ambiente de trabalho para explorar e editar bases de dados SQLite, desenvolvida em Rust com [egui](https://github.com/emilk/egui).

## Funcionalidades

- **Abrir bases de dados** — ficheiros `.db`, `.sqlite`, `.sqlite3`
- **Separador Dados** — navegar, filtrar, ordenar e paginar registos; editar, inserir e eliminar registos diretamente
- **Separador Esquema** — visualizar a instrução SQL CREATE de cada tabela
- **Separador Estatísticas** — estatísticas por coluna (total de linhas, nulos, valores únicos, mín/máx/média, valores mais frequentes); exportar para JSON
- **Editor SQL** — executar consultas livres, autocompletar palavras-chave e nomes de tabelas/colunas, atalho Ctrl+Enter, histórico e consultas favoritas
- **Importar CSV** — carregar um ficheiro CSV para uma nova tabela
- **Exportar CSV / JSON** — exportar a vista atual ou os resultados de uma consulta SQL
- **Criar tabela** — diálogo visual com definição de colunas e suporte a chaves estrangeiras
- **Renomear / eliminar tabelas** — via menu de contexto (clique direito)
- **Vista ERD** — diagrama entidade-relacionamento com tabelas e chaves estrangeiras
- **Compactar** — otimizar e reduzir o tamanho do ficheiro de base de dados (VACUUM)
- **Tema Escuro / Claro** — alternável em tempo de execução
- **Interface multilingue** — Português e Inglês

## Requisitos

- [Rust](https://rustup.rs/) 1.75 ou superior

## Compilar

```bash
cargo build --release
```

O binário é gerado em `target/release/sqlite-workbench-rs`.

## Executar

```bash
cargo run --release
```

Ou executar o binário diretamente e abrir uma base de dados em **Ficheiro → Carregar base de dados**.

## Atalhos de teclado

| Atalho | Ação |
|--------|------|
| `Ctrl+Enter` | Executar consulta SQL |

## Definições

As definições (idioma, tema, linhas por página) são guardadas em `settings.json` junto ao binário. O histórico de consultas é guardado em `history.json` e os favoritos em `favorites.json`.

## Licença

Licenciado ao abrigo da [Licença Pública da União Europeia v1.2](LICENSE.pt-PT) (EUPL-1.2).
