// Conteggi per la home, dal DB locale (record eliminati esclusi).

use crate::db;
use crate::secure_store::err_to_string;
use crate::session;
use rusqlite::Row;
use serde::Serialize;

const COUNTS_SQL: &str = "SELECT (SELECT COUNT(*) FROM Patient WHERE PatientDeleted = 0), (SELECT COUNT(*) FROM Visit WHERE VDeleted = 0)";

#[derive(Serialize)]
pub struct Counts {
	patients: i64,
	visits: i64,
}

#[tauri::command]
pub fn get_counts() -> Result<Counts, String> {
	session::require_unlocked()?;
	let guard = db::lock();
	let connection = db::get(&guard)?;
	connection.query_row(COUNTS_SQL, [], row_to_counts).map_err(err_to_string)
}

fn row_to_counts(row: &Row) -> rusqlite::Result<Counts> {
	Ok(Counts { patients: row.get(0)?, visits: row.get(1)? })
}