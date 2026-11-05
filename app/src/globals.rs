// Port di AppGlobal.swift: host ammessi, host preferito, endpoint.

use crate::secure_store;
use serde::Serialize;

pub const HOSTS: [&str; 4] = [
	"www.optigest.cloud",
	"www.optigest.app",
	"www.optigest.me",
	"www.eosgest.com",
];

const LOGIN_SIGNUP_ENDPOINT: &str = "/app/login.php?setup=workstation&api=app";
const TEST_LOGIN_ENDPOINT: &str = "/app/php/testLogin.php?app=true";

#[derive(Serialize)]
pub struct HostsInfo {
	hosts: Vec<String>,
	current: String,
}

#[tauri::command]
pub fn get_hosts() -> HostsInfo {
	let hosts: Vec<String> = HOSTS.iter().map(host_to_string).collect();
	HostsInfo { hosts, current: backend_domain() }
}

#[tauri::command]
pub fn set_host(host: String) -> Result<(), String> {
	if !select_preferred_host(&host) { return Err(format!("Host non ammesso: {}", host)); }
	Ok(())
}

pub fn backend_domain() -> String {
	match secure_store::get(secure_store::KEY_HOST) {
		Some(host) if HOSTS.contains(&host.as_str()) => host,
		_ => HOSTS[0].to_string(),
	}
}

pub fn backend_host() -> String {
	format!("https://{}", backend_domain())
}

pub fn login_signup_url() -> String {
	format!("{}{}", backend_host(), LOGIN_SIGNUP_ENDPOINT)
}

pub fn test_login_url() -> String {
	format!("{}{}", backend_host(), TEST_LOGIN_ENDPOINT)
}

pub fn select_preferred_host(host: &str) -> bool {
	if !HOSTS.contains(&host) { return false; }
	secure_store::set(secure_store::KEY_HOST, host).is_ok()
}

fn host_to_string(host: &&str) -> String {
	host.to_string()
}
