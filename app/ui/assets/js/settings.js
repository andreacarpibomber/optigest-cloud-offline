// Impostazioni: tema, modifica PIN, sblocco con impronta.
// Dipende da theme.js (getThemeMode, setThemeMode) e common.js (invoke, $).

let biometricEnabled = false;

document.addEventListener("page-ready", onReady);

function onReady() {
	initTheme();
	$("pin-form").addEventListener("submit", submitPinChange);
	loadBiometric();
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