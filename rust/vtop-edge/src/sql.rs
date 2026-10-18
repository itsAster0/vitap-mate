//! SQL storage for the Durable Objects. Each object keeps its own tables,
//! which the dashboard's data viewer can read (the key-value API hides
//! everything in an internal table it cannot).
//!
//! An object made before the tables existed still holds key-value data:
//! [`needs_schema`] tells its owner to carry over what it keeps, wipe the
//! rest and create the tables with [`create`].

use serde_json::{Map, Value};
use worker::{Result, SqlStorage, SqlStorageValue};

pub type Row = Map<String, Value>;

/// Bumped when a table changes; [`create`] records it.
const SCHEMA_VERSION: i64 = 1;

pub fn rows(db: &SqlStorage, query: &str, binds: Vec<SqlStorageValue>) -> Result<Vec<Row>> {
    db.exec(query, binds)?.to_array::<Row>()
}

pub fn first(db: &SqlStorage, query: &str, binds: Vec<SqlStorageValue>) -> Result<Option<Row>> {
    Ok(rows(db, query, binds)?.into_iter().next())
}

pub fn run(db: &SqlStorage, query: &str, binds: Vec<SqlStorageValue>) -> Result<()> {
    rows(db, query, binds).map(|_| ())
}

/// Whether the tables are still to be made.
pub fn needs_schema(db: &SqlStorage) -> Result<bool> {
    Ok(first(
        db,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
        vec![],
    )?
    .is_none())
}

/// Runs `schema` (statements separated by `;`) and marks the object done.
pub fn create(db: &SqlStorage, schema: &str) -> Result<()> {
    for statement in schema.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        run(db, statement, vec![])?;
    }
    run(
        db,
        "CREATE TABLE IF NOT EXISTS meta (schema_version INTEGER NOT NULL)",
        vec![],
    )?;
    run(db, "DELETE FROM meta", vec![])?;
    run(
        db,
        "INSERT INTO meta (schema_version) VALUES (?)",
        vec![SCHEMA_VERSION.into()],
    )
}

pub fn text(row: &Row, column: &str) -> Option<String> {
    row.get(column).and_then(Value::as_str).map(str::to_string)
}

/// Integers come back from SQLite as JavaScript numbers.
pub fn int(row: &Row, column: &str) -> Option<u64> {
    row.get(column)
        .and_then(Value::as_f64)
        .filter(|value| *value >= 0.0)
        .map(|value| value as u64)
}

pub fn flag(row: &Row, column: &str) -> bool {
    int(row, column).is_some_and(|value| value != 0)
}

pub fn v_text(value: &str) -> SqlStorageValue {
    value.into()
}

pub fn v_opt_text(value: Option<&str>) -> SqlStorageValue {
    value.into()
}

/// Times and counters fit easily in JavaScript's safe integers.
pub fn v_int(value: u64) -> SqlStorageValue {
    SqlStorageValue::Integer(value as i64)
}

pub fn v_opt_int(value: Option<u64>) -> SqlStorageValue {
    value.map_or(SqlStorageValue::Null, v_int)
}

pub fn v_flag(value: bool) -> SqlStorageValue {
    SqlStorageValue::Integer(i64::from(value))
}
