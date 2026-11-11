// Ingresso: splash -> sessione salvata? NO -> pair
//                                       SI -> PIN presente? SI -> sblocco / NO -> imposta PIN
//        -> homepage.html (dopo il PIN)
// Dipende da common.js (invoke, $, sleep).

const listen = window.__TAURI__.event.listen;

const MIN_SPLASH_MS = 1800;
const HOST_LONG_PRESS_MS = 3000;
const HOME_PAGE = "homepage.html";

let pinMode = "unlock";
let hostTimer = null;
// la richiesta dell'impronta parte da sola una volta per apertura; poi solo dal pulsante
let biometricTried = false;
let biometricEnabled = false;

function show(viewId) {
	document.querySelectorAll(".view").forEach(hideView);
	$(viewId).classList.add("active");
}

function hideView(view) {
	view.classList.remove("active");
}

async function start() {
	if (await invoke("is_unlocked")) { openHome(); return; }
	show("view-splash");
	$("splash-note").textContent = "";
	$("btn-repair").classList.add("hidden");
	const began = Date.now();
	await loadHost();
	const state = await invoke("boot_state");
	const remaining = Math.max(0, MIN_SPLASH_MS - (Date.now() - began));
	await sleep(remaining);
	route(state);
}

function route(state) {
	if (state === "pair") { startPair(); return; }
	if (state === "set_pin") { showPin("set"); return; }
	showPin("unlock");
}

async function startPair() {
	show("view-splash");
	$("splash-note").textContent = "Completa l'accesso a OptiGest Cloud nella finestra aperta.";
	$("btn-repair").classList.remove("hidden");
	try {
		await invoke("start_pair");
	} catch (error) {
		$("splash-note").textContent = "Impossibile aprire l'accesso: " + error;
	}
}

function showPin(mode) {
	pinMode = mode;
	show("view-pin");
	$("pin-error").textContent = "";
	$("pin-input").value = "";
	$("pin-confirm").value = "";
	if (mode === "set") {
		$("pin-title").textContent = "Imposta il PIN di sblocco";
		$("pin-confirm").classList.remove("hidden");
	} else {
		$("pin-title").textContent = "Inserisci il PIN";
		$("pin-confirm").classList.add("hidden");
	}
	$("pin-input").focus();
	$("btn-biometric").classList.add("hidden");
	if (mode === "unlock") { offerBiometric(); }
}

// Icona visibile se il computer ha un lettore. Se l'impronta è attiva la richiesta parte da sola
// (una volta per apertura); se fallisce o viene annullata resta il PIN.
async function offerBiometric() {
	let status;
	try {
		status = await invoke("biometric_status");
	} catch (error) {
		console.error("biometric_status", error);
		return;
	}
	biometricEnabled = status.enabled;
	if (!status.available) { return; }
	$("btn-biometric").classList.remove("hidden");
	if (status.enabled && !biometricTried) { biometricTried = true; tryBiometric(); }
}

async function tryBiometric() {
	if (!biometricEnabled) {
		$("pin-error").textContent = "Sblocca con il PIN, poi attiva l'impronta dalla home.";
		return;
	}
	let unlocked = false;
	try {
		unlocked = await invoke("unlock_biometric");
	} catch (error) {
		$("pin-error").textContent = String(error);
		return;
	}
	if (unlocked) { openHome(); return; }
	$("pin-input").focus();
}

async function submitPin(event) {
	event.preventDefault();
	const pin = $("pin-input").value;
	if (pinMode === "set") { await submitNewPin(pin); return; }
	await submitUnlock(pin);
}

async function submitNewPin(pin) {
	if (pin !== $("pin-confirm").value) {
		$("pin-error").textContent = "I due PIN non coincidono.";
		return;
	}
	try {
		await invoke("set_pin", { pin: pin });
	} catch (error) {
		$("pin-error").textContent = String(error);
		return;
	}
	openHome();
}

async function submitUnlock(pin) {
	let outcome;
	try {
		outcome = await invoke("verify_pin", { pin: pin });
	} catch (error) {
		$("pin-error").textContent = String(error);
		return;
	}
	if (outcome.status === "ok") { openHome(); return; }
	if (outcome.status === "locked") { await start(); return; }
	$("pin-input").value = "";
	$("pin-error").textContent = "PIN errato. Tentativi rimasti: " + outcome.remaining;
}

function openHome() {
	location.replace(HOME_PAGE);
}

// Versione dell'app (Cargo.toml), mostrata in fondo alla pagina di accesso.
async function loadVersion() {
	try {
		$("app-version").textContent = "Versione " + await invoke("app_version");
	} catch (error) {
		console.error("app_version", error);
	}
}

// Host corrente (da config.json): mostrato sullo splash, cliccabile per cambiarlo.
async function loadHost() {
	const info = await invoke("get_hosts");
	$("splash-host").textContent = info.current + " ▾";
}

// Selettore host: click sul nome dell'host, oppure long press 3s sul logo dello splash (come la fascia in alto nell'app Swift).
async function openHostPicker() {
	const info = await invoke("get_hosts");
	const list = $("host-list");
	list.innerHTML = "";
	info.hosts.forEach(function (host) {
		const item = document.createElement("li");
		const button = document.createElement("button");
		button.type = "button";
		button.textContent = host === info.current ? host + " ✓" : host;
		button.addEventListener("click", function () { chooseHost(host); });
		item.appendChild(button);
		list.appendChild(item);
	});
	$("host-overlay").classList.remove("hidden");
}

async function chooseHost(host) {
	await invoke("set_host", { host: host });
	await loadHost();
	$("host-overlay").classList.add("hidden");
}

function logoPressStart() {
	hostTimer = setTimeout(openHostPicker, HOST_LONG_PRESS_MS);
}

function logoPressEnd() {
	clearTimeout(hostTimer);
}

$("pin-form").addEventListener("submit", submitPin);
$("btn-repair").addEventListener("click", startPair);
$("btn-biometric").addEventListener("click", tryBiometric);
$("btn-support").addEventListener("click", function () { invoke("open_support").catch(function (error) { $("pin-error").textContent = String(error); }); });
$("host-cancel").addEventListener("click", function () { $("host-overlay").classList.add("hidden"); });
$("logo").addEventListener("pointerdown", logoPressStart);
$("logo").addEventListener("pointerup", logoPressEnd);
$("logo").addEventListener("pointerleave", logoPressEnd);
const hostLabel = $("splash-host");
hostLabel.setAttribute("role", "button");
hostLabel.setAttribute("tabindex", "0");
hostLabel.addEventListener("click", openHostPicker);
hostLabel.addEventListener("keydown", function (event) { if (event.key === "Enter" || event.key === " ") openHostPicker(); });

listen("pair-completed", start);

start();
loadVersion();
