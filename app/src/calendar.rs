// Appuntamenti dal DB locale (sola lettura) per il calendario.
// Le date sono quelle del cloud ("YYYY-MM-DD HH:MM:SS"): si confrontano come testo e non si convertono.

use crate::db;
use crate::secure_store::err_to_string;
use rusqlite::{named_params, Row};
use serde::Serialize;

const MAX_EVENTS: i64 = 5000;

const LIST_SQL: &str = "SELECT EventID, EventOwnerID, EventTitle, EventStart, EventEnd, EventData
	FROM Event
	WHERE EventDeleted = 0 AND EventStart < :end AND EventEnd >= :start
	ORDER BY EventStart, EventID
	LIMIT :limit";

#[derive(Serialize)]
pub struct EventRow {
	id: i64,
	owner_id: Option<i64>,
	title: String,
	start: String,
	end: String,
	// JSON di EventData (testo): lo interpreta la pagina
	data: String,
}

// Appuntamenti che si sovrappongono all'intervallo [start, end).
pub fn list(start: String, end: String) -> Result<Vec<EventRow>, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let mut statement = connection.prepare(LIST_SQL).map_err(err_to_string)?;
	let found = statement
		.query_map(named_params! { ":start": start, ":end": end, ":limit": MAX_EVENTS }, row_to_event)
		.map_err(err_to_string)?;
	let mut events: Vec<EventRow> = Vec::new();
	for row in found { events.push(row.map_err(err_to_string)?); }
	Ok(events)
}

fn row_to_event(row: &Row) -> rusqlite::Result<EventRow> {
	Ok(EventRow { id: row.get(0)?, owner_id: row.get(1)?, title: row.get(2)?, start: row.get(3)?, end: row.get(4)?, data: row.get(5)? })
}