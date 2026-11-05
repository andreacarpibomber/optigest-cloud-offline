#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod globals;
mod logdb;
mod pair;
mod pin;
mod secure_store;
mod session;

fn main() {
	tauri::Builder::default()
		.setup(setup_app)
		.invoke_handler(tauri::generate_handler![
			session::boot_state,
			session::connect_upstream,
			session::logout,
			pin::set_pin,
			pin::verify_pin,
			pair::start_pair,
			globals::get_hosts,
			globals::set_host,
			logdb::list_logs,
		])
		.run(tauri::generate_context!())
		.expect("errore durante l'avvio di OptiGest Cloud Offline");
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	config::init(app.handle())?;
	logdb::init(app.handle())?;
	let _ = logdb::write(&format!("Avvio applicazione (host: {})", globals::backend_domain()));
	Ok(())
}