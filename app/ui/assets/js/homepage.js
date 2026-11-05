// Homepage: saluto, widget con i conteggi (DB locale) e risposta del login per il debug dello stile.

document.addEventListener("page-ready", onReady);

function onReady(event) {
	const info = event.detail;
	$("home-greeting").textContent = greeting() + " " + (info.title || "Dott.") + " " + info.name;
	$("home-raw").textContent = info.raw || "(nessuna risposta: offline, mostrati i dati in cache)";
	loadCounts();
}

// Conteggi dei record non eliminati: funzionano anche offline.
async function loadCounts() {
	try {
		const counts = await invoke("get_counts");
		$("count-patients").textContent = formatNumber(counts.patients);
		$("count-visits").textContent = formatNumber(counts.visits);
	} catch (error) {
		$("count-patients").textContent = "?";
		$("count-visits").textContent = "?";
	}
}

function greeting() {
	const hour = new Date().getHours();
	if (hour < 13) { return "Buon giorno"; }
	if (hour < 18) { return "Buon pomeriggio"; }
	return "Buona sera";
}