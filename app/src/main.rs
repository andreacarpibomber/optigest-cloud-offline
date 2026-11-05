#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod config;
mod db;
#[cfg(debug_assertions)]
mod dev_store;
mod globals;
mod logdb;
mod pair;
mod patients;
mod pin;
mod profile;
mod secure_store;
mod session;
mod stats;

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
			api::start_pair,
			api::get_hosts,
			api::set_host,
			api::get_counts,
			api::list_logs,
			api::search_patients,
			api::get_patient,
		])
		.run(tauri::generate_context!())
		.expect("errore durante l'avvio di OptiGest Cloud Offline");
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	secure_store::init(app.handle())?;
	config::init(app.handle())?;
	db::init(app.handle())?;
	let _ = logdb::write(&format!("Avvio applicazione (host: {})", globals::backend_domain()));
	Ok(())
}