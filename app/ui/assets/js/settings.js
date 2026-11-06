// Impostazioni: tema, modifica PIN, sblocco con impronta.
// Dipende da theme.js (getThemeMode, setThemeMode) e common.js (invoke, $).

let biometricEnabled = false;

document.addEventListener("page-ready", onReady);

function onReady() {
	initTheme();
	$("pin-form").addEventListener("submit", submitPinChange);
	loadBiometric();
	loadBackup();
}

// ---- Tema ----

function initTheme() {
	document.querySelectorAll("#theme-choice button").forEach((button) => {
		button.addEventListener("click", () => { setThemeMode(button.dataset.mode); showTheme(); });
	});
	showTheme();
}

function showTheme() {
	const mode = getThemeMode();
	document.querySelectorAll("#theme-choice button").forEach((button) => {
		button.classList.toggle("active", button.dataset.mode === mode);
	});
}

// ---- PIN ----

async function submitPinChange(event) {
	event.preventDefault();
	const current = $("pin-current").value;
	const next = $("pin-new").value;
	if (next !== $("pin-repeat").value) { pinMessage("I due PIN nuovi non coincidono.", true); return; }
	let outcome;
	try {
		outcome = await invoke("change_pin", { currentPin: current, newPin: next });
	} catch (error) {
		pinMessage(String(error), true);
		return;
	}
	if (outcome.status === "locked") { location.replace("index.html"); return; }
	if (outcome.status === "wrong") {
		pinMessage("PIN attuale errato. Tentativi rimasti: " + outcome.remaining, true);
		return;
	}
	$("pin-form").reset();
	pinMessage("PIN modificato.", false);
}

function pinMessage(text, isError) {
	const node = $("pin-message");
	node.textContent = text;
	node.classList.toggle("error", isError);
}

// ---- Impronta ----

async function loadBiometric() {
	let status;
	try {
		status = await invoke("biometric_status");
	} catch (error) {
		return;
	}
	if (!status.available) { return; }
	biometricEnabled = status.enabled;
	$("biometric-section").hidden = false;
	$("btn-biometric").addEventListener("click", toggleBiometric);
	showBiometric();
}

function showBiometric() {
	$("biometric-status").textContent = biometricEnabled ? "Attivo: all'apertura puoi usare l'impronta al posto del PIN." : "Non attivo: si usa il PIN.";
	$("btn-biometric").textContent = biometricEnabled ? "Disattiva" : "Attiva";
}

async function toggleBiometric() {
	const button = $("btn-biometric");
	button.disabled = true;
	try {
		await invoke("set_biometric", { enabled: !biometricEnabled });
		biometricEnabled = !biometricEnabled;
		showBiometric();
	} catch (error) {
		$("biometric-status").textContent = String(error);
	}
	button.disabled = false;
}

// ---- Backup del DB locale ----

async function loadBackup() {
	$("backup-keep").addEventListener("change", changeBackupKeep);
	$("btn-backup").addEventListener("click", runBackup);
	try {
		showBackup(await invoke("get_backup_status"));
	} catch (error) {
		backupMessage(String(error), true);
	}
}

function showBackup(status) {
	const select = $("backup-keep");
	select.replaceChildren();
	for (let n = status.min_keep; n <= status.max_keep; n++) {
		const option = element("option", "", String(n));
		option.value = String(n);
		option.selected = n === status.keep;
		select.append(option);
	}
	const error = $("backup-error");
	error.hidden = !status.last_error;
	error.textContent = status.last_error ? "Ultimo backup non riuscito: " + status.last_error : "";
	const list = $("backup-list");
	list.replaceChildren();
	if (status.backups.length === 0) { list.append(element("li", "", "Nessun backup ancora.")); }
	status.backups.forEach((backup) => {
		list.append(element("li", "", formatDateTime(backup.created_on) + " · " + formatSize(backup.size_bytes)));
	});
}

function formatSize(bytes) {
	if (bytes >= 1048576) { return (bytes / 1048576).toFixed(1) + " MB"; }
	return Math.max(1, Math.round(bytes / 1024)) + " KB";
}

async function changeBackupKeep() {
	try {
		showBackup(await invoke("set_backup_keep", { keep: Number($("backup-keep").value) }));
		backupMessage("Impostazione salvata.", false);
	} catch (error) {
		backupMessage(String(error), true);
	}
}

async function runBackup() {
	const button = $("btn-backup");
	button.disabled = true;
	backupMessage("Backup in corso...", false);
	try {
		const outcome = await invoke("run_backup_now");
		backupMessage(outcome.done ? "Backup eseguito." : outcome.message, false);
	} catch (error) {
		backupMessage(String(error), true);
	}
	try {
		showBackup(await invoke("get_backup_status"));
	} catch (error) {
		backupMessage(String(error), true);
	}
	button.disabled = false;
}

function backupMessage(text, isError) {
	const node = $("backup-message");
	node.textContent = text;
	node.classList.toggle("error", isError);
}