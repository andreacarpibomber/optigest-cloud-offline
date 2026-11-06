// Condiviso da tutte le pagine. Le pagine interne hanno <body class="page" data-page="...">:
// all'apertura verificano lo sblocco (PIN), caricano i dati di sessione e disegnano la barra in alto.
// Quando è pronto, scatta l'evento "page-ready" con i dati di sessione in event.detail.

const invoke = window.__TAURI__.core.invoke;

const SESSION_KEY = "optigest.session";
const LOGIN_PAGE = "index.html";

const FOOTER_LINKS = [
	{ page: "security", label: "Sicurezza" },
	{ page: "legal", label: "Legale" },
	{ page: "privacy", label: "Privacy" },
];

const ICON_LOCK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>';
const ICON_COPYRIGHT = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M14.5 9.5a3 3 0 1 0 0 5"/></svg>';

const NAV = [
	{ page: "homepage", label: "Home", href: "homepage.html" },
	{ page: "patients", label: "Pazienti", href: "patients.html" },
	{ page: "visits", label: "Visite", href: "visits.html" },
	{ page: "settings", label: "Impostazioni", href: "settings.html" },
];

function $(id) {
	return document.getElementById(id);
}

function sleep(ms) {
	return new Promise(function (resolve) { setTimeout(resolve, ms); });
}

// 15480 -> 15'480 (separatore a scelta: nelle tabelle si usa il punto, 2.614)
function formatNumber(value, separator) {
	return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, separator || "'");
}

// "1990-05-01" -> "01/05/1990"
function formatDate(value) {
	if (!value) { return ""; }
	const parts = value.split("-");
	if (parts.length !== 3) { return value; }
	return parts[2] + "/" + parts[1] + "/" + parts[0];
}

// Il DB salva "YYYY-MM-DD HH:MM:SS" in UTC: si mostra nell'ora locale.
function formatDateTime(value) {
	if (!value) { return ""; }
	const date = new Date(value.replace(" ", "T") + "Z");
	if (isNaN(date.getTime())) { return value; }
	return date.toLocaleString("it-IT");
}

async function initPage() {
	const page = document.body.dataset.page;
	if (!page) { return; }
	if (!(await invoke("is_unlocked"))) { goToLogin(); return; }
	const forceRefresh = document.body.dataset.refresh === "true";
	const info = await loadSession(forceRefresh);
	if (!info) { return; }
	const host = await invoke("get_hosts");
	renderTopbar(page, info, host.current);
	renderFooter(info.online);
	document.dispatchEvent(new CustomEvent("page-ready", { detail: info }));
}

// Dati di sessione (studio, utente, online): sessionStorage tra le pagine, cloud solo se mancano o se richiesto.
async function loadSession(forceRefresh) {
	if (!forceRefresh) {
		const stored = readStored();
		if (stored) { return stored; }
	}
	let info;
	try {
		info = await invoke("connect_upstream");
	} catch (error) {
		goToLogin();
		return null;
	}
	if (!info.valid) { goToLogin(); return null; }
	writeStored(info);
	return info;
}

function readStored() {
	try {
		const text = sessionStorage.getItem(SESSION_KEY);
		if (text) { return JSON.parse(text); }
	} catch (error) {
		return null;
	}
	return null;
}

function writeStored(info) {
	try {
		sessionStorage.setItem(SESSION_KEY, JSON.stringify(info));
	} catch (error) {
		// senza storage si rifà la chiamata a ogni pagina
	}
}

function clearStored() {
	try {
		sessionStorage.removeItem(SESSION_KEY);
	} catch (error) {
		// niente da cancellare
	}
}

function goToLogin() {
	clearStored();
	location.replace(LOGIN_PAGE);
}

async function logout() {
	await invoke("logout");
	goToLogin();
}

function renderTopbar(page, info, host) {
	const bar = $("topbar");
	if (!bar) { return; }
	bar.classList.add("topbar");
	bar.replaceChildren();
	bar.append(buildLogo());
	bar.append(buildNav(page));
	bar.append(element("span", "topbar-spacer", ""));
	bar.append(buildStatus(info.online, host));
	bar.append(buildLogoutButton());
}

// Footer comune a tutte le pagine interne: link legali (aperti nel browser di sistema), stato connessione, copyright.
function renderFooter(online) {
	if ($("footer")) { return; }
	const footer = document.createElement("footer");
	footer.id = "footer";
	footer.className = "page-footer";
	footer.append(buildFooterLinks());
	const meta = element("div", "footer-meta", "");
	meta.append(buildFooterNote(ICON_LOCK, online ? "Connessione sicura" : "Modalità offline: dati locali"));
	meta.append(buildFooterNote(ICON_COPYRIGHT, "EosMedTech Srl"));
	footer.append(meta);
	document.body.append(footer);
}

function buildFooterLinks() {
	const links = element("div", "footer-links", "");
	for (const item of FOOTER_LINKS) {
		const link = element("a", "", item.label);
		link.href = "#";
		link.addEventListener("click", function (event) {
			event.preventDefault();
			invoke("open_legal_page", { page: item.page }).catch(function () { /* nessun browser disponibile: niente da fare */ });
		});
		links.append(link);
	}
	return links;
}

// Le icone sono SVG fissi del codice (niente font esterni: l'app funziona anche offline).
function buildFooterNote(icon, text) {
	const note = document.createElement("span");
	note.innerHTML = icon;
	note.append(text);
	return note;
}

function buildLogo() {
	const link = element("a", "topbar-logo", "");
	link.href = "homepage.html";
	const image = document.createElement("img");
	image.src = "assets/img/logo.svg";
	image.alt = "EosMedTech";
	image.height = 44;
	link.append(image);
	return link;
}

function buildNav(page) {
	const nav = element("nav", "topbar-nav", "");
	for (const item of NAV) {
		const link = element("a", item.page === page ? "active" : "", item.label);
		link.href = item.href;
		nav.append(link);
	}
	return nav;
}

function buildStatus(online, host) {
	const status = element("div", "status " + (online ? "online" : "offline"), "");
	status.append(document.createElement("i"));
	status.append(element("span", "", (online ? "Online" : "Offline") + " · " + host));
	return status;
}

function buildLogoutButton() {
	const button = element("button", "link", "Disconnetti");
	button.type = "button";
	button.addEventListener("click", logout);
	return button;
}

function element(tag, className, text) {
	const node = document.createElement(tag);
	if (className) { node.className = className; }
	node.textContent = text;
	return node;
}

// Dopo DOMContentLoaded tutti gli script della pagina (es. homepage.js) hanno già registrato i loro listener.
document.addEventListener("DOMContentLoaded", initPage);