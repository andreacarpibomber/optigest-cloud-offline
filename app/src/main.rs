#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod backup;
mod biometric;
mod config;
mod db;
#[cfg(debug_assertions)]
mod dev_store;
mod external;
mod globals;
mod httplog;
mod license;
mod logdb;
mod pair;
mod patients;
mod pin;
mod profile;
mod secure_store;
mod session;
mod stats;
mod sync;
mod visits;

fn main() {
	tauri::Builder::default()
		.setup(setup_app)
		.invoke_handler(tauri::generate_handler![
			api::boot_state,
			api::is_unlocked,
			api::logout,
			api::connect_upstream,
			api::set_pin,
			api::verify_pin,
			api::change_pin,
			api::biometric_status,
			api::set_biometric,
			api::unlock_biometric,
			api::start_pair,
			api::get_hosts,
			api::set_host,
				api::open_legal_page,
			api::get_counts,
			api::list_logs,
			api::search_patients,
			api::get_patient,
			api::search_visits,
			api::get_visit,
			api::sync_now,
			api::get_sync_status,
			api::get_license_status,
			api::refresh_license,
			api::get_backup_status,
			api::set_backup_keep,
			api::run_backup_now,
		])
		.run(tauri::generate_context!())
		.expect("errore durante l'avvio di OptiGest Cloud Offline");
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	secure_store::init(app.handle())?;
	config::init(app.handle())?;
	db::init(app.handle())?;
	backup::init(app.handle())?;
	let _ = logdb::write(&format!("Avvio applicazione (host: {})", globals::backend_domain()));
	Ok(())
}