// Configurazione (config.json).
// Il file app/config.json (accanto a Cargo.toml) è incorporato nel binario e contiene i valori di default.
// Se l'host cambia a runtime (selettore host o pair), la scelta è salvata in un config.json
// nella cartella di configurazione dell'utente e ha la precedenza sul file incorporato.

use crate::secure_store::err_to_string;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager};

pub const DEFAULT_HOST: &str = "www.optigest.app";

const CONFIG_FILE: &str = "config.json";
const BUNDLED: &str = include_str!("../config.json");

#[derive(Serialize, Deserialize)]
struct Config {
	#[serde(default = "default_host")]
	host: String,
}

struct State {
	override_path: PathBuf,
	config: Config,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub fn init(app: &AppHandle) -> Result<(), String> {
	let dir = app.path().app_config_dir().map_err(err_to_string)?;
	fs::create_dir_all(&dir).map_err(err_to_string)?;
	let override_path = dir.join(CONFIG_FILE);
	let config = match read_file(&override_path) {
		Some(config) => config,
		None => bundled(),
	};
	*lock() = Some(State { override_path, config });
	Ok(())
}

pub fn host() -> String {
	match lock().as_ref() {
		Some(state) => state.config.host.clone(),
		None => DEFAULT_HOST.to_string(),
	}
}

// Salva la scelta nel file dell'utente (il file incorporato non è scrivibile).
pub fn set_host(host: &str) -> Result<(), String> {
	let mut guard = lock();
	let state = match guard.as_mut() {
		Some(state) => state,
		None => return Err("Configurazione non inizializzata".to_string()),
	};
	state.config.host = host.to_string();
	let json = serde_json::to_string_pretty(&state.config).map_err(err_to_string)?;
	fs::write(&state.override_path, json).map_err(err_to_string)
}

fn read_file(path: &Path) -> Option<Config> {
	let content = fs::read_to_string(path).ok()?;
	serde_json::from_str(&content).ok()
}

fn bundled() -> Config {
	match serde_json::from_str(BUNDLED) {
		Ok(config) => config,
		Err(_) => Config { host: default_host() },
	}
}

fn default_host() -> String {
	DEFAULT_HOST.to_string()
}

fn lock() -> MutexGuard<'static, Option<State>> {
	STATE.lock().unwrap_or_else(PoisonError::into_inner)
}