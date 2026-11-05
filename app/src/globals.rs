// Port di AppGlobal.swift: host ammessi, host preferito, endpoint.

use crate::config;
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

pub fn get_hosts() -> HostsInfo {
	let hosts: Vec<String> = HOSTS.iter().map(host_to_string).collect();
	HostsInfo { hosts, current: backend_domain() }
}

pub fn set_host(host: String) -> Result<(), String> {
	if !select_preferred_host(&host) { return Err(format!("Host non ammesso: {}", host)); }
	Ok(())
}

// Host da config.json; se non è tra quelli ammessi si usa il default.
pub fn backend_domain() -> String {
	let host = config::host();
	if HOSTS.contains(&host.as_str()) { return host; }
	config::DEFAULT_HOST.to_string()
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
	config::set_host(host).is_ok()
}

fn host_to_string(host: &&str) -> String {
	host.to_string()
}