// Scheda paziente (sola lettura): patient.html?id=...
// Dati a sinistra, prossimi appuntamenti e visite svolte a destra (le visite sono in patient-visits.js).

const GENDERS = { M: "Maschio", F: "Femmina", A: "Altro" };
const GENDER_TITLES = { M: "Sig.", F: "Sig.ra" };
const NATIONS = { IT: "Italia" };

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
	const title = renderTitle(patient);
	renderFields(patient);
	addWordButton("patient-title", function () { return patientWord(patient, title); });
	await renderAppointments(id);
	forceRepaint();
}

// "Sig. Pepe Diego Gabriele (37)": titolo e età come nel cloud
function renderTitle(patient) {
	const name = (patient.surname + " " + patient.name).trim() || "Paziente " + patient.id;
	const parts = [];
	if (GENDER_TITLES[patient.gender]) { parts.push(GENDER_TITLES[patient.gender]); }
	parts.push(name);
	const age = ageFrom(patient.birthday);
	if (age !== null) { parts.push("(" + age + ")"); }
	const title = parts.join(" ");
	$("patient-title").textContent = title;
	return title;
}

function patientWord(patient, title) {
	const address = [patient.address, patient.postal_code, patient.city, patient.province, NATIONS[patient.nation] || patient.nation].filter(Boolean).join(", ");
	return {
		filename: "Riepilogo paziente " + patient.surname + " " + patient.name,
		title: "Riepilogo dati paziente - " + title,
		sections: [{ title: "", rows: [
			["Nome", patient.name], ["Cognome", patient.surname], ["Genere", GENDERS[patient.gender] || "Non impostato"],
			["Data di nascita", formatDate(patient.birthday)], ["Luogo di nascita", joinPlace(patient.birth_place, patient.birth_province)],
			["Codice Fiscale", patient.fiscal_code], ["Indirizzo", address], ["Recapiti telefonici", parsePhones(patient.telephones).join(" - ")],
			["Email", patient.email], ["Ultimo aggiornamento", formatDateTime(patient.last_update)],
		] }],
	};
}

function renderFields(patient) {
	const phones = parsePhones(patient.telephones);
	setValue("p-surname", patient.surname);
	setValue("p-name", patient.name);
	setValue("p-gender", GENDERS[patient.gender] || "Non impostato");
	setValue("p-birthday", formatDate(patient.birthday));
	setValue("p-birth-place", joinPlace(patient.birth_place, patient.birth_province));
	setValue("p-fiscal", patient.fiscal_code);
	setValue("p-phone1", phones[0]);
	setValue("p-phone2", phones[1]);
	setValue("p-email", patient.email);
	setValue("p-nation", NATIONS[patient.nation] || patient.nation);
	setValue("p-postal", patient.postal_code);
	setValue("p-city", patient.city);
	setValue("p-province", patient.province);
	setValue("p-address", patient.address);
	setValue("p-current", patient.current_practitioner);
	setValue("p-sending", patient.sending_practitioner);
	setValue("p-general", patient.general_anamnesis);
	setValue("p-ocular", patient.ocular_anamnesis);
	setValue("p-allergies", patient.allergies);
	setValue("p-info", patient.info);
	setValue("p-consent", formatDate((patient.consent_date || "").slice(0, 10)));
	setValue("p-id", String(patient.id));
	setValue("p-created", formatDateTime(patient.created_on));
	setValue("p-updated", formatDateTime(patient.last_update));
	renderTags(patient.tags);
}

function setValue(id, value) {
	$(id).textContent = value || "";
}

function renderTags(text) {
	const box = $("p-tags");
	box.replaceChildren();
	for (const tag of parseTags(text)) { box.append(element("span", "tag", tag)); }
}

// Il DB conserva i tag come testo: elenco JSON oppure separato da virgole.
function parseTags(text) {
	if (!text) { return []; }
	try {
		const list = JSON.parse(text);
		if (Array.isArray(list)) { return list.map(String).filter(Boolean); }
	} catch (error) {
		// non è JSON: si divide sulle virgole
	}
	return text.split(",").map(function (tag) { return tag.trim(); }).filter(Boolean);
}

// Il DB conserva i telefoni come testo JSON (array); se non lo è si mostra così com'è.
function parsePhones(text) {
	if (!text) { return []; }
	try {
		const list = JSON.parse(text);
		if (Array.isArray(list)) { return list.map(String); }
	} catch (error) {
		return [text];
	}
	return [text];
}

function joinPlace(place, province) {
	if (!place) { return ""; }
	if (!province) { return place; }
	return place + " - " + province;
}

// Età in anni compiuti, o null se la data manca o non è valida
function ageFrom(birthday) {
	if (!birthday) { return null; }
	const born = new Date(birthday + "T00:00:00");
	if (isNaN(born.getTime())) { return null; }
	const today = new Date();
	let age = today.getFullYear() - born.getFullYear();
	const hadBirthday = today.getMonth() > born.getMonth() || (today.getMonth() === born.getMonth() && today.getDate() >= born.getDate());
	if (!hadBirthday) { age -= 1; }
	return age >= 0 ? age : null;
}

// ---- Appuntamenti (prossimi) ----
async function renderAppointments(patientId) {
	const list = $("appointments-list");
	list.replaceChildren();
	let events;
	try {
		events = await invoke("list_patient_events", { patientId: patientId });
	} catch (error) {
		list.append(element("li", "", "Appuntamenti non disponibili: " + String(error)));
		return;
	}
	if (events.length === 0) { list.append(element("li", "", "Non è stato prenotato alcun appuntamento")); return; }
	for (const event of events) { list.append(appointmentItem(event)); }
}

// "07/11/2026 08:15 - titolo": clic -> calendario in quel giorno
function appointmentItem(event) {
	const item = element("li", "event-li", formatDate(event.start.slice(0, 10)) + " " + event.start.slice(11, 16) + (event.title ? " - " + event.title : ""));
	item.addEventListener("click", function () { location.href = "calendar.html?date=" + encodeURIComponent(event.start.slice(0, 10)); });
	return item;
}

// Nella WebView le card possono restare dipinte "a metà" quando l'altezza cambia dopo il caricamento dei dati:
// si nascondono e si rimostrano per forzare un ridisegno completo.
function forceRepaint() {
	requestAnimationFrame(function () {
		const cards = document.querySelectorAll(".detail-card, .table-card");
		cards.forEach(function (card) { card.style.display = "none"; });
		void document.body.offsetHeight;
		cards.forEach(function (card) { card.style.display = ""; });
	});
}

function showError(message) {
	$("patient-error").textContent = message;
}
