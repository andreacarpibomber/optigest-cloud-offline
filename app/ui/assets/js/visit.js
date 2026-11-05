// Scheda visita (sola lettura): visit.html?id=...

document.addEventListener("page-ready", onReady);

async function onReady() {
	const id = Number(new URLSearchParams(location.search).get("id"));
	if (!Number.isInteger(id) || id <= 0) { showError("Visita non valida."); return; }
	let visit;
	try {
		visit = await invoke("get_visit", { id: id });
	} catch (error) {
		showError(String(error));
		return;
	}
	$("visit-title").textContent = "Visita del " + formatDate(visit.date.slice(0, 10));
	renderFields(visit);
	renderData(visit.data);
}

function renderFields(visit) {
	const list = $("visit-fields");
	list.replaceChildren();
	addField(list, "ID visita", String(visit.id));
	addField(list, "Data", formatDate(visit.date.slice(0, 10)));
	addPatientField(list, visit);
	addField(list, "Motivo", visit.cause);
	addField(list, "Conclusioni", visit.diagnosis);
	addField(list, "Ultimo aggiornamento", formatDateTime(visit.last_update));
}

// Il paziente è un link alla sua scheda.
function addPatientField(list, visit) {
	const name = (visit.patient_surname + " " + visit.patient_name).trim() || "Paziente " + visit.patient_id;
	const link = element("a", "", name);
	link.href = "patient.html?id=" + encodeURIComponent(visit.patient_id);
	list.append(element("dt", "", "Paziente"));
	const value = element("dd", "", "");
	value.append(link);
	list.append(value);
}

function addField(list, label, value) {
	if (!value) { return; }
	list.append(element("dt", "", label));
	list.append(element("dd", "", value));
}

// I dati sono JSON; se non lo fossero si mostrano così come sono.
function renderData(text) {
	if (!text) { return; }
	let shown = text;
	try {
		shown = JSON.stringify(JSON.parse(text), null, 2);
	} catch (error) {
		shown = text;
	}
	$("visit-data").textContent = shown;
	$("visit-data-box").hidden = false;
}

function showError(message) {
	$("visit-error").textContent = message;
}