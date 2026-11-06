// Tema chiaro/scuro/sistema. Caricato nell'<head> delle pagine interne per evitare il lampo di colore.
// La scelta è salvata in localStorage (se non disponibile resta "sistema").

const THEME_KEY = "optigest.theme";
const THEME_MODES = ["system", "light", "dark"];
const THEME_QUERY = window.matchMedia("(prefers-color-scheme: dark)");

function getThemeMode() {
	try {
		const mode = localStorage.getItem(THEME_KEY);
		return THEME_MODES.indexOf(mode) >= 0 ? mode : "system";
	} catch (error) {
		return "system";
	}
}

function applyTheme() {
	const mode = getThemeMode();
	const dark = mode === "dark" || (mode === "system" && THEME_QUERY.matches);
	document.documentElement.dataset.theme = dark ? "dark" : "light";
}

function setThemeMode(mode) {
	try {
		localStorage.setItem(THEME_KEY, mode);
	} catch (error) {
		// senza storage vale solo per questa pagina
	}
	applyTheme();
}

THEME_QUERY.addEventListener("change", applyTheme);
applyTheme();