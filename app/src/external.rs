// Apertura di pagine web nel browser di sistema (link del footer: Sicurezza, Legale, Privacy).
// Il webview dell'app non apre da solo i link target="_blank".

use std::process::Command;

pub fn open(url: &str) -> Result<(), String> {
	let mut command = open_command(url);
	command.spawn().map(|_| ()).map_err(|error| format!("Impossibile aprire il browser: {}", error))
}

#[cfg(target_os = "macos")]
fn open_command(url: &str) -> Command {
	let mut command = Command::new("open");
	command.arg(url);
	command
}

#[cfg(target_os = "windows")]
fn open_command(url: &str) -> Command {
	let mut command = Command::new("cmd");
	command.args(["/C", "start", "", url]);
	command
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_command(url: &str) -> Command {
	let mut command = Command::new("xdg-open");
	command.arg(url);
	command
}