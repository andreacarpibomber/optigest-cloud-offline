// Router unico delle API: tutti i comandi che la UI chiama con invoke("nome", {...}) stanno qui.
// I moduli (session, pin, patients, ...) contengono solo la logica; qui si decide cosa è esposto e a quali condizioni.
//
//   Pubbliche (anche con app bloccata): boot_state, is_unlocked, set_pin, verify_pin, start_pair, get_hosts, set_host, logout
//   Protette (richiedono lo sblocco con PIN): connect_upstream, get_counts, list_logs, search_patients, get_patient
//
// Per aggiungere un'API: funzione qui con #[tauri::command], poi la riga in generate_handler! di main.rs.

use crate::globals::{self, HostsInfo};
use crate::logdb::{self, LogEntry};
use crate::pair;
use crate::patients::{self, PatientDetail, PatientPage, PatientQuery};
use crate::pin::{self, PinOutcome};
use crate::session::{self, UpstreamInfo};
use crate::stats::{self, Counts};
use tauri::AppHandle;

// ---- Sessione ----

#[tauri::command]
pub fn boot_state() -> &'static str {
	session::boot_state()
}

#[tauri::command]
pub fn is_unlocked() -> bool {
	session::is_unlocked()
}

#[tauri::command]
pub fn logout() {
	session::logout();
}

#[tauri::command]
pub async fn connect_upstream() -> Result<UpstreamInfo, String> {
	session::require_unlocked()?;
	session::connect_upstream().await
}

// ---- PIN e pair ----

#[tauri::command]
pub fn set_pin(pin: String) -> Result<(), String> {
	pin::set_pin(pin)
}

#[tauri::command]
pub fn verify_pin(pin: String) -> Result<PinOutcome, String> {
	pin::verify_pin(pin)
}

#[tauri::command]
pub async fn start_pair(app: AppHandle) -> Result<(), String> {
	pair::start_pair(app).await
}

// ---- Host ----

#[tauri::command]
pub fn get_hosts() -> HostsInfo {
	globals::get_hosts()
}

#[tauri::command]
pub fn set_host(host: String) -> Result<(), String> {
	globals::set_host(host)
}

// ---- Dati locali (protette) ----

#[tauri::command]
pub fn get_counts() -> Result<Counts, String> {
	session::require_unlocked()?;
	stats::get_counts()
}

#[tauri::command]
pub fn list_logs(limit: Option<u32>) -> Result<Vec<LogEntry>, String> {
	session::require_unlocked()?;
	logdb::list_logs(limit)
}

#[tauri::command]
pub fn search_patients(query: PatientQuery) -> Result<PatientPage, String> {
	session::require_unlocked()?;
	patients::search(query)
}

#[tauri::command]
pub fn get_patient(id: i64) -> Result<PatientDetail, String> {
	session::require_unlocked()?;
	patients::get(id)
}