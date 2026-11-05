// Scheda paziente (sola lettura): patient.html?id=...

document.addEventListener("page-ready", onReady);

async function onReady() {
	const id = Number(new URLSearchParams(location.search).get("id"));
	if (!Number.isInteger(id) || id <= 0) { showError("Paziente non valido."); return; }
	let patient;
	try {
		patient = await invoke("get_patient", { id: id });
	} catch (error) {
		showError(String(error));
		return;
	}
	$("patient-title").textContent = (patient.surname + " " + patient.name).trim() || "Paziente " + patient.id;
	renderFields(patient);
}

function renderFields(patient) {
	const fields = [
		["ID paziente", String(patient.id)],
		["Data di nascita", formatDate(patient.birthday)],
		["Luogo di nascita", withProvince(patient.birth_place, patient.birth_province)],
		["Sesso", patient.gender],
		["Codice fiscale", patient.fiscal_code],
		["Email", patient.email],
		["Telefoni", formatPhones(patient.telephones)],
		["Indirizzo", patient.address],
		["Città", withProvince((patient.postal_code + " " + patient.city).trim(), patient.province)],
		["Nazione", patient.nation],
		["Inviante", patient.sending_practitioner],
		["Medico curante", patient.current_practitioner],
		["Tag", patient.tags],
		["Allergie", patient.allergies],
		["Anamnesi generale", patient.general_anamnesis],
		["Anamnesi oculare", patient.ocular_anamnesis],
		["Note", patient.info],
		["Creato il", formatDateTime(patient.created_on)],
		["Ultimo aggiornamento", formatDateTime(patient.last_update)],
	];
	const list = $("patient-fields");
	list.replaceChildren();
	for (const [label, value] of fields) {
		if (!value) { continue; }
		list.append(element("dt", "", label));
		list.append(element("dd", "", value));
	}
}

function withProvince(place, province) {
	if (!place) { return ""; }
	if (!province) { return place; }
	return place + " (" + province + ")";
}

// Il DB conserva i telefoni come testo JSON (array); se non lo è si mostra così com'è.
function formatPhones(text) {
	if (!text) { return ""; }
	try {
		const list = JSON.parse(text);
		if (Array.isArray(list)) { return list.join(", "); }
	} catch (error) {
		return text;
	}
	return text;
}

function showError(message) {
	$("patient-error").textContent = message;
}