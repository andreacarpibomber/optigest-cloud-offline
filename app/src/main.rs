#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod db;
#[cfg(debug_assertions)]
mod dev_store;
mod globals;
mod logdb;
mod pair;
mod pin;
mod profile;
mod secure_store;
mod session;
mod stats;

fn main() {
	tauri::Builder::default()
		.setup(setup_app)
		.invoke_handler(tauri::generate_handler![
			session::boot_state,
			session::connect_upstream,
			session::logout,
			session::is_unlocked,
			pin::set_pin,
			pin::verify_pin,
			pair::start_pair,
			globals::get_hosts,
			globals::set_host,
			logdb::list_logs,
			stats::get_counts,
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