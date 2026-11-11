#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod autosync;
mod backup;
mod biometric;
mod calendar;
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
mod word;

// Argomento con cui il sistema avvia l'app al login: parte nascosta, solo nella barra di sistema.
const BACKGROUND_ARG: &str = "--background";
const MAIN_WINDOW: &str = "main";
const AUTOSTART_MARKER: &str = "autostart_initialized";

fn main() {
	let app = tauri::Builder::default()
		// Una sola istanza: aprire di nuovo l'app riporta davanti quella già in esecuzione (e il suo scheduler)
		.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
		.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![BACKGROUND_ARG])))
		.setup(setup_app)
		.on_window_event(on_window_event)
		.invoke_handler(tauri::generate_handler![
			api::boot_state,
			api::app_version,
			api::set_zoom,
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
			api::open_cloud,
			api::open_cloud_calendar,
			api::open_cloud_record,
			api::set_host,
			api::open_legal_page,
			api::get_counts,
			api::list_logs,
			api::search_patients,
			api::get_patient,
			api::search_visits,
			api::get_visit,
			api::export_word,
			api::list_events,
			api::list_patient_events,
			api::sync_now,
			api::get_sync_status,
			api::get_license_status,
			api::refresh_license,
			api::get_backup_status,
			api::set_backup_keep,
			api::run_backup_now,
		])
		.build(tauri::generate_context!())
		.expect("errore durante l'avvio di OptiGest Cloud Offline");

	app.run(|handle, event| match event {
		// Chiusa l'ultima finestra l'app resta attiva (sincronizzazione in background); si esce dal menu "Esci"
		tauri::RunEvent::ExitRequested { api, code, .. } => {
			if code.is_none() { api.prevent_exit(); }
		}
		// macOS: clic sull'icona nel Dock
		#[cfg(target_os = "macos")]
		tauri::RunEvent::Reopen { .. } => show_main_window(handle),
		_ => {
			let _ = handle;
		}
	});
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	secure_store::init(app.handle())?;
	config::init(app.handle())?;
	db::init(app.handle())?;
	backup::init(app.handle())?;
	let _ = logdb::write(&format!("Avvio applicazione (host: {})", globals::backend_domain()));
	enable_autostart_once(app);
	setup_tray(app)?;
	autosync::start(app.handle().clone());
	let background = std::env::args().any(|argument| argument == BACKGROUND_ARG);
	if !background { show_main_window(app.handle()); }
	Ok(())
}

// Avvio al login attivato una sola volta (se l'utente lo disattiva dalle impostazioni del sistema non si riattiva).
fn enable_autostart_once(app: &tauri::App) {
	use tauri::Manager;
	use tauri_plugin_autostart::ManagerExt;
	let dir = match app.path().app_data_dir() {
		Ok(dir) => dir,
		Err(_) => return,
	};
	let marker = dir.join(AUTOSTART_MARKER);
	if marker.exists() { return; }
	if app.autolaunch().enable().is_ok() {
		let _ = std::fs::write(&marker, "1");
		let _ = logdb::write("Avvio automatico al login attivato");
	}
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	use tauri::menu::{Menu, MenuItem};
	use tauri::tray::TrayIconBuilder;
	let open = MenuItem::with_id(app, "open", "Apri OptiGest", true, None::<&str>)?;
	let quit = MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?;
	let menu = Menu::with_items(app, &[&open, &quit])?;
	let mut builder = TrayIconBuilder::new().menu(&menu).tooltip("OptiGest Cloud Offline").on_menu_event(on_tray_menu);
	if let Some(icon) = app.default_window_icon() { builder = builder.icon(icon.clone()); }
	builder.build(app)?;
	Ok(())
}

fn on_tray_menu(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
	match event.id().as_ref() {
		"open" => show_main_window(app),
		"quit" => app.exit(0),
		_ => {}
	}
}

// Chiudere la finestra non chiude l'app: la nasconde e la blocca (al ritorno serve di nuovo il PIN). 
fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    use tauri::Manager;
    if window.label() != MAIN_WINDOW {
        return;
    }
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        hide_main_window(window.app_handle());
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
	use tauri::Manager;
	session::set_unlocked(false);
	if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
		let _ = window.eval("window.location.replace('index.html')");
		let _ = window.hide();
	}
}

fn show_main_window(app: &tauri::AppHandle) {
	use tauri::Manager;
	if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
		let _ = window.show();
		let _ = window.unminimize();
		let _ = window.set_focus();
	}
}
