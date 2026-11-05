#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod globals;
mod pair;
mod pin;
mod secure_store;
mod session;

fn main() {
	tauri::Builder::default()
		.invoke_handler(tauri::generate_handler![
			session::boot_state,
			session::connect_upstream,
			session::logout,
			pin::set_pin,
			pin::verify_pin,
			pair::start_pair,
			globals::get_hosts,
			globals::set_host,
		])
		.run(tauri::generate_context!())
		.expect("errore durante l'avvio di OptiGest Cloud Offline");
}
