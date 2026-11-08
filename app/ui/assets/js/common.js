// Condiviso da tutte le pagine. Le pagine interne hanno <body class="page" data-page="...">:
// all'apertura verificano lo sblocco (PIN), caricano i dati di sessione e disegnano la barra in alto.
// Quando è pronto, scatta l'evento "page-ready" con i dati di sessione in event.detail.

const invoke = window.__TAURI__.core.invoke;

// Layout con menu laterale (assets/css/shell.css): caricato qui così le pagine non cambiano.
(function loadShellStyle() {
	const link = document.createElement("link");
	link.rel = "stylesheet";
	link.href = "assets/css/shell.css";
	document.head.append(link);
})();

const SESSION_KEY = "optigest.session";
const LOGIN_PAGE = "index.html";

const FOOTER_LINKS = [
	{ page: "security", label: "Sicurezza" },
	{ page: "legal", label: "Legale" },
	{ page: "privacy", label: "Privacy" },
];

const ICON_LOCK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>';
const ICON_COPYRIGHT = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M14.5 9.5a3 3 0 1 0 0 5"/></svg>';

const ICON_HOME = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3h12a1 1 0 0 1 1 1v17l-7-4-7 4V4a1 1 0 0 1 1-1z"/></svg>';
const ICON_LIST = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 6h11M10 12h11M10 18h11"/><path d="M3 6h.01M3 12h.01M3 18h.01"/></svg>';
const ICON_SETTINGS = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>';
const ICON_USER = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="4"/><path d="M4 21v-1a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v1"/></svg>';
const ICON_LOGOUT = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/></svg>';

// Menu laterale: sezioni come nel cloud. Le pagine di dettaglio evidenziano la voce dell'elenco.
const NAV_SECTIONS = [
	{
		title: "Pannello studio",
		items: [
			{ page: "homepage", label: "Home", href: "homepage.html", icon: ICON_HOME },
			{ page: "patients", label: "Pazienti", href: "patients.html", icon: ICON_LIST },
			{ page: "visits", label: "Visite", href: "visits.html", icon: ICON_LIST },
		],
	},
	{
		title: "Impostazioni",
		items: [
			{ page: "settings", label: "Impostazioni", href: "settings.html", icon: ICON_SETTINGS },
		],
	},
];
const PAGE_PARENT = { patient: "patients", visit: "visits" };

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
	if ((await invoke("get_license_status")).blocked) { location.replace("license.html"); return; }
	const forceRefresh = document.body.dataset.refresh === "true";
	const info = await loadSession(forceRefresh);
	if (!info) { return; }
	const host = await invoke("get_hosts");
	renderShell(page, info, host.current);
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

// Menu laterale + barra in alto (stato e utente). Dipende da assets/css/shell.css.
function renderShell(page, info, host) {
	document.body.classList.add("shell");
	if (!$("sidebar")) { document.body.prepend(buildSidebar(PAGE_PARENT[page] || page, info.online)); }
	const bar = $("topbar");
	if (!bar) { return; }
	bar.classList.add("topbar");
	bar.replaceChildren();
	bar.append(element("span", "topbar-spacer", ""));
	bar.append(buildStatus(info.online, host));
	bar.append(buildUserMenu(info));
}

function buildSidebar(page, online) {
	const sidebar = document.createElement("aside");
	sidebar.id = "sidebar";
	sidebar.className = "sidebar";
	sidebar.append(buildBrand());
	sidebar.append(buildNav(page));
	sidebar.append(buildSidebarFooter(online));
	return sidebar;
}

// Logo bianco (assets/img/logo_w.svg, quello del cloud); se manca si usa quello scuro reso chiaro dal CSS.
function buildBrand() {
	const link = element("a", "sidebar-brand", "");
	link.href = "homepage.html";
	const image = document.createElement("img");
	image.src = "assets/img/logo_w.svg";
	image.alt = "OptiGest";
	image.addEventListener("error", function () {
		image.classList.add("logo-invert");
		image.src = "assets/img/logo.svg";
	}, { once: true });
	link.append(image);
	return link;
}

function buildNav(page) {
	const nav = element("nav", "sidebar-nav", "");
	for (const section of NAV_SECTIONS) {
		nav.append(element("div", "nav-title", section.title));
		for (const item of section.items) { nav.append(buildNavItem(item, item.page === page)); }
	}
	return nav;
}

function buildNavItem(item, active) {
	const link = element("a", "nav-item" + (active ? " active" : ""), "");
	link.href = item.href;
	link.innerHTML = item.icon;
	link.append(element("span", "", item.label));
	return link;
}

// In fondo al menu: link legali (browser di sistema), stato connessione, copyright.
function buildSidebarFooter(online) {
	const footer = element("div", "sidebar-footer", "");
	footer.append(buildFooterLinks());
	footer.append(buildFooterNote(ICON_LOCK, online ? "Connessione sicura" : "Modalità offline: dati locali"));
	footer.append(buildFooterNote(ICON_COPYRIGHT, "EosMedTech Srl"));
	return footer;
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
	const note = element("span", "footer-note", "");
	note.innerHTML = icon;
	note.append(text);
	return note;
}

function buildStatus(online, host) {
	const status = element("div", "status " + (online ? "online" : "offline"), "");
	status.append(document.createElement("i"));
	status.append(element("span", "", (online ? "Online" : "Offline") + " · " + host));
	return status;
}

// Nome utente + avatar: al click si apre il menu con Impostazioni e Disconnetti.
function buildUserMenu(info) {
	const wrapper = element("div", "user-menu", "");
	const button = element("button", "user-button", "");
	button.type = "button";
	button.append(element("span", "user-name", ((info.title ? info.title + " " : "") + (info.name || "")).trim() || "Utente"));
	const avatar = element("span", "user-avatar", "");
	avatar.innerHTML = ICON_USER;
	button.append(avatar);
	const dropdown = element("div", "user-dropdown", "");
	dropdown.hidden = true;
	dropdown.append(buildMenuLink("Impostazioni", ICON_SETTINGS, function () { location.href = "settings.html"; }));
	dropdown.append(buildMenuLink("Disconnetti", ICON_LOGOUT, logout));
	button.addEventListener("click", function (event) {
		event.stopPropagation();
		dropdown.hidden = !dropdown.hidden;
	});
	document.addEventListener("click", function () { dropdown.hidden = true; });
	wrapper.append(button, dropdown);
	return wrapper;
}

function buildMenuLink(label, icon, action) {
	const item = element("button", "dropdown-item", "");
	item.type = "button";
	item.innerHTML = icon;
	item.append(element("span", "", label));
	item.addEventListener("click", action);
	return item;
}

function element(tag, className, text) {
	const node = document.createElement(tag);
	if (className) { node.className = className; }
	node.textContent = text;
	return node;
}

// Dopo DOMContentLoaded tutti gli script della pagina (es. homepage.js) hanno già registrato i loro listener.
document.addEventListener("DOMContentLoaded", initPage);