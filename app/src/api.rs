// Router unico delle API: tutti i comandi che la UI chiama con invoke("nome", {...}) stanno qui.
// I moduli (session, pin, patients, ...) contengono solo la logica; qui si decide cosa è esposto e a quali condizioni.
//
//   Pubbliche (anche con app bloccata): boot_state, is_unlocked, set_pin, verify_pin, biometric_status, unlock_biometric, start_pair, get_hosts, set_host, open_legal_page, logout
//   Protette (richiedono lo sblocco con PIN): connect_upstream, get_counts, list_logs,
//   search_patients, get_patient, search_visits, get_visit, sync_now, get_sync_status
//
// Per aggiungere un'API: funzione qui con #[tauri::command], poi la riga in generate_handler! di main.rs.

use crate::backup::{self, BackupOutcome, BackupStatus};
use crate::license;
use crate::biometric::{self, BiometricStatus};
use crate::external;
use crate::globals::{self, HostsInfo};
use crate::logdb::{self, LogEntry};
use crate::pair;
use crate::patients::{self, PatientDetail, PatientPage, PatientQuery};
use crate::pin::{self, PinOutcome};
use crate::session::{self, UpstreamInfo};
use crate::stats::{self, Counts};
use crate::sync::{self, SyncReport, SyncStatus};
use crate::visits::{self, VisitDetail, VisitPage, VisitQuery};
use tauri::{AppHandle, Emitter};

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
pub fn change_pin(current: String, new_pin: String) -> Result<PinOutcome, String> {
	session::require_unlocked()?;
	pin::change_pin(current, new_pin)
}

#[tauri::command]
pub fn verify_pin(pin: String) -> Result<PinOutcome, String> {
	pin::verify_pin(pin)
}

// Impronta (pubbliche per la schermata del PIN; l'attivazione richiede l'app sbloccata)
#[tauri::command]
pub fn biometric_status() -> BiometricStatus {
	biometric::status()
}

#[tauri::command]
pub async fn set_biometric(enabled: bool) -> Result<(), String> {
	biometric::set_enabled(enabled).await
}

#[tauri::command]
pub async fn unlock_biometric() -> Result<bool, String> {
	biometric::unlock().await
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

// Apre nel browser di sistema una pagina del sito (security, legal, privacy).
#[tauri::command]
pub fn open_legal_page(page: String) -> Result<(), String> {
	let url = globals::legal_url(&page).ok_or("Pagina non valida")?;
	external::open(&url)
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

#[tauri::command]
pub fn search_visits(query: VisitQuery) -> Result<VisitPage, String> {
	session::require_unlocked()?;
	visits::search(query)
}

#[tauri::command]
pub fn get_visit(id: i64) -> Result<VisitDetail, String> {
	session::require_unlocked()?;
	visits::get(id)
}

// ---- Sincronizzazione (protette) ----

// Avanzamento sull'evento "sync-progress" ({ table, rows }). Errore SESSION_EXPIRED = token non più valido.
#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<SyncReport, String> {
	session::require_unlocked()?;
	sync::run(&move |progress| { let _ = app.emit("sync-progress", progress); }).await
}

#[tauri::command]
pub fn get_sync_status() -> Result<SyncStatus, String> {
	session::require_unlocked()?;
	sync::status()
}

// ---- Backup del DB locale (protette) ----

#[tauri::command]
pub fn get_backup_status() -> Result<BackupStatus, String> {
	session::require_unlocked()?;
	backup::status()
}

#[tauri::command]
pub fn set_backup_keep(keep: u32) -> Result<BackupStatus, String> {
	session::require_unlocked()?;
	backup::set_keep(keep)
}

// Backup manuale. Il lavoro (integrità, copia, verifica) gira fuori dal thread dell'interfaccia.
#[tauri::command]
pub async fn run_backup_now() -> Result<BackupOutcome, String> {
	session::require_unlocked()?;
	tauri::async_runtime::spawn_blocking(|| backup::run(true)).await.map_err(|error| error.to_string())?
}

// ---- Licenza (pubbliche: servono anche a app bloccata per licenza) ----

#[tauri::command]
pub fn get_license_status() -> license::LicenseStatus {
	license::status()
}

#[tauri::command]
pub async fn refresh_license() -> Result<license::LicenseStatus, String> {
	license::refresh().await
}