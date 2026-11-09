// Visite dal DB locale: ricerca con filtri per colonna (anche sui dati del paziente), ordinamento, paginazione,
// e scheda della singola visita. Le visite eliminate sono escluse. Come in patients.rs i valori cercati passano
// sempre come parametri e l'ORDER BY viene da una lista fissa.

use crate::db;
use crate::secure_store::err_to_string;
use rusqlite::{named_params, Row};
use serde::{Deserialize, Serialize};

const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 100;

const VISIT_FILTER_SQL: &str = "v.VDeleted = 0
	AND (:patient_id = 0 OR v.VPatientID = :patient_id)
	AND (:id = '' OR CAST(v.VisitID AS TEXT) LIKE '%' || :id || '%' ESCAPE '\\')
	AND (:date = '' OR substr(v.VDate, 1, 10) = :date)
	AND (:cause = '' OR v.VCause LIKE '%' || :cause || '%' ESCAPE '\\')
	AND (:diagnosis = '' OR v.VDiagnosis LIKE '%' || :diagnosis || '%' ESCAPE '\\')";

const PATIENT_FILTER_SQL: &str = "(:surname = '' OR p.PatientSurname LIKE '%' || :surname || '%' ESCAPE '\\')
	AND (:name = '' OR p.PatientName LIKE '%' || :name || '%' ESCAPE '\\')
	AND (:birthday = '' OR p.PatientBirthday = :birthday)
	AND (:fiscal_code = '' OR p.PatientFiscalCode LIKE '%' || :fiscal_code || '%' ESCAPE '\\')";

// Senza filtri sul paziente il conteggio non ha bisogno del JOIN (con molte visite è molto più veloce).
const NO_PATIENT_FILTER_SQL: &str = "(:surname = '' AND :name = '' AND :birthday = '' AND :fiscal_code = '')";

const LIST_COLUMNS: &str = "v.VisitID, v.VPatientID, v.VDate, COALESCE(p.PatientSurname, ''), COALESCE(p.PatientName, ''),
	p.PatientBirthday, COALESCE(p.PatientFiscalCode, ''), v.VCause, v.VDiagnosis";

const FROM_JOIN_SQL: &str = "FROM Visit v LEFT JOIN Patient p ON p.PatientID = v.VPatientID";

const DETAIL_SQL: &str = "SELECT v.VisitID, v.VPatientID, COALESCE(p.PatientSurname, ''), COALESCE(p.PatientName, ''),
	v.VDate, v.VCause, v.VDiagnosis, v.VData, v.VLastUpdate
	FROM Visit v LEFT JOIN Patient p ON p.PatientID = v.VPatientID
	WHERE v.VisitID = :id AND v.VDeleted = 0";

#[derive(Deserialize)]
#[serde(default)]
pub struct VisitQuery {
	// 0 = tutti i pazienti; altrimenti solo le visite di quel paziente (scheda paziente)
	pub patient_id: i64,
	pub id: String,
	// "YYYY-MM-DD"
	pub date: String,
	pub surname: String,
	pub name: String,
	// "YYYY-MM-DD"
	pub birthday: String,
	pub fiscal_code: String,
	pub cause: String,
	pub diagnosis: String,
	// "id" | "date" | "surname" | "name" | "birthday" | "fiscal_code" | "cause" | "diagnosis"
	pub sort: String,
	pub desc: bool,
	// da 1
	pub page: u32,
	pub page_size: u32,
}

impl Default for VisitQuery {
	fn default() -> VisitQuery {
		VisitQuery {
			patient_id: 0,
			id: String::new(),
			date: String::new(),
			surname: String::new(),
			name: String::new(),
			birthday: String::new(),
			fiscal_code: String::new(),
			cause: String::new(),
			diagnosis: String::new(),
			sort: "id".to_string(),
			desc: true,
			page: 1,
			page_size: DEFAULT_PAGE_SIZE,
		}
	}
}

#[derive(Serialize)]
pub struct VisitRow {
	id: i64,
	patient_id: i64,
	date: String,
	surname: String,
	name: String,
	birthday: Option<String>,
	fiscal_code: String,
	cause: String,
	diagnosis: String,
}

#[derive(Serialize)]
pub struct VisitPage {
	rows: Vec<VisitRow>,
	total: i64,
	page: u32,
	page_size: u32,
}

#[derive(Serialize)]
pub struct VisitDetail {
	id: i64,
	patient_id: i64,
	patient_surname: String,
	patient_name: String,
	date: String,
	cause: String,
	diagnosis: String,
	// testo JSON così com'è nel DB
	data: String,
	last_update: String,
}

pub fn search(query: VisitQuery) -> Result<VisitPage, String> {
	let page = query.page.max(1);
	let page_size = if query.page_size == 0 { DEFAULT_PAGE_SIZE } else { query.page_size.min(MAX_PAGE_SIZE) };
	let offset = (page as i64 - 1) * page_size as i64;
	let id = escape_like(query.id.trim());
	let date = query.date.trim().to_string();
	let surname = escape_like(query.surname.trim());
	let name = escape_like(query.name.trim());
	let birthday = query.birthday.trim().to_string();
	let fiscal_code = escape_like(query.fiscal_code.trim());
	let cause = escape_like(query.cause.trim());
	let diagnosis = escape_like(query.diagnosis.trim());
	let has_patient_filter = !(surname.is_empty() && name.is_empty() && birthday.is_empty() && fiscal_code.is_empty());

	let list_sql = format!(
		"SELECT {} {} WHERE {} AND {} ORDER BY {} LIMIT :limit OFFSET :offset",
		LIST_COLUMNS,
		FROM_JOIN_SQL,
		VISIT_FILTER_SQL,
		PATIENT_FILTER_SQL,
		order_by(&query.sort, query.desc)
	);
	let count_sql = if has_patient_filter {
		format!("SELECT COUNT(*) {} WHERE {} AND {}", FROM_JOIN_SQL, VISIT_FILTER_SQL, PATIENT_FILTER_SQL)
	} else {
		format!("SELECT COUNT(*) FROM Visit v WHERE {} AND {}", VISIT_FILTER_SQL, NO_PATIENT_FILTER_SQL)
	};

	let guard = db::lock();
	let connection = db::get(&guard)?;
	let total: i64 = connection
		.query_row(
			&count_sql,
			named_params! {
				":patient_id": query.patient_id, ":id": id, ":date": date, ":cause": cause, ":diagnosis": diagnosis,
				":surname": surname, ":name": name, ":birthday": birthday, ":fiscal_code": fiscal_code
			},
			read_total,
		)
		.map_err(err_to_string)?;
	let mut statement = connection.prepare(&list_sql).map_err(err_to_string)?;
	let found = statement
		.query_map(
			named_params! {
				":patient_id": query.patient_id, ":id": id, ":date": date, ":cause": cause, ":diagnosis": diagnosis,
				":surname": surname, ":name": name, ":birthday": birthday, ":fiscal_code": fiscal_code,
				":limit": page_size, ":offset": offset
			},
			row_to_visit,
		)
		.map_err(err_to_string)?;
	let mut rows: Vec<VisitRow> = Vec::new();
	for row in found { rows.push(row.map_err(err_to_string)?); }
	Ok(VisitPage { rows, total, page, page_size })
}

pub fn get(id: i64) -> Result<VisitDetail, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	match connection.query_row(DETAIL_SQL, named_params! { ":id": id }, row_to_detail) {
		Ok(detail) => Ok(detail),
		Err(rusqlite::Error::QueryReturnedNoRows) => Err("Visita non trovata".to_string()),
		Err(error) => Err(err_to_string(error)),
	}
}

// Parti fisse e controllate: il testo ricevuto sceglie solo tra queste voci.
// Il tie-break su VisitID rende l'ordine stabile tra pagine.
fn order_by(sort: &str, desc: bool) -> String {
	let column = match sort {
		"date" => "v.VDate",
		"surname" => "p.PatientSurname COLLATE NOCASE",
		"name" => "p.PatientName COLLATE NOCASE",
		"birthday" => "p.PatientBirthday",
		"fiscal_code" => "p.PatientFiscalCode COLLATE NOCASE",
		"cause" => "v.VCause COLLATE NOCASE",
		"diagnosis" => "v.VDiagnosis COLLATE NOCASE",
		_ => "v.VisitID",
	};
	let direction = if desc { "DESC" } else { "ASC" };
	format!("{} {}, v.VisitID DESC", column, direction)
}

// % e _ cercati dall'utente valgono come caratteri normali (ESCAPE '\').
fn escape_like(text: &str) -> String {
	text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn read_total(row: &Row) -> rusqlite::Result<i64> {
	row.get(0)
}

fn row_to_visit(row: &Row) -> rusqlite::Result<VisitRow> {
	Ok(VisitRow {
		id: row.get(0)?,
		patient_id: row.get(1)?,
		date: row.get(2)?,
		surname: row.get(3)?,
		name: row.get(4)?,
		birthday: row.get(5)?,
		fiscal_code: row.get(6)?,
		cause: row.get(7)?,
		diagnosis: row.get(8)?,
	})
}

fn row_to_detail(row: &Row) -> rusqlite::Result<VisitDetail> {
	Ok(VisitDetail {
		id: row.get(0)?,
		patient_id: row.get(1)?,
		patient_surname: row.get(2)?,
		patient_name: row.get(3)?,
		date: row.get(4)?,
		cause: row.get(5)?,
		diagnosis: row.get(6)?,
		data: row.get(7)?,
		last_update: row.get(8)?,
	})
}