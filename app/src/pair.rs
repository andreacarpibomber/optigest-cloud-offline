// Pair con OptiGest Cloud: stesso flusso di LoginSignupViewController.swift.
// Finestra separata con dati non persistenti (incognito) che carica login.php?setup=workstation.
// Il server risponde navigando verso app://loginTkn=... oppure app://host=...: qui vengono intercettati.

use crate::globals;
use crate::logdb;
use crate::secure_store;
use crate::secure_store::err_to_string;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

const PAIR_LABEL: &str = "pair";
const PREFIX_TOKEN: &str = "app://loginTkn=";
const PREFIX_HOST: &str = "app://host=";

#[tauri::command]
pub async fn start_pair(app: AppHandle) -> Result<(), String> {
	if let Some(existing) = app.get_webview_window(PAIR_LABEL) {
		let _ = existing.set_focus();
		return Ok(());
	}
	let url = Url::parse(&globals::login_signup_url()).map_err(err_to_string)?;
	let handle = app.clone();
	WebviewWindowBuilder::new(&app, PAIR_LABEL, WebviewUrl::External(url))
		.title("Associa a OptiGest Cloud")
		.inner_size(480.0, 720.0)
		.center()
		.incognito(true)
		.on_navigation(move |target| allow_navigation(&handle, target))
		.build()
		.map_err(err_to_string)?;
	Ok(())
}

// true = la navigazione prosegue, false = bloccata (gestita qui).
fn allow_navigation(app: &AppHandle, target: &Url) -> bool {
	let raw = target.as_str();
	if let Some(host) = raw.strip_prefix(PREFIX_HOST) {
		globals::select_preferred_host(host);
		return false;
	}
	if let Some(token) = raw.strip_prefix(PREFIX_TOKEN) {
		complete_pairing(app, token);
		return false;
	}
	true
}

fn complete_pairing(app: &AppHandle, token: &str) {
	if token.is_empty() { return; }
	if secure_store::set(secure_store::KEY_TOKEN, token).is_err() { return; }
	let _ = logdb::write("Associazione a OptiGest Cloud completata");
	// Chiusura fuori dal callback di navigazione, per evitare deadlock del webview.
	let handle = app.clone();
	tauri::async_runtime::spawn(async move {
		close_pair_window(&handle);
		let _ = handle.emit_to("main", "pair-completed", ());
	});
}

fn close_pair_window(app: &AppHandle) {
	if let Some(window) = app.get_webview_window(PAIR_LABEL) { let _ = window.close(); }
}