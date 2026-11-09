// Visite svolte dal paziente (scheda paziente). Clic sulla riga -> visit.html?id=...
// La logica della tabella è in table.js; qui si aggiunge solo il filtro sul paziente.

const visitsPatientId = Number(new URLSearchParams(location.search).get("id"));

if (Number.isInteger(visitsPatientId) && visitsPatientId > 0) {
	new DataTable({
		command: "search_visits",
		stateKey: "optigest.patient-visits." + visitsPatientId,
		defaultSort: "date",
		actions: false,
		filters: {
			date: "f-date",
			cause: "f-cause",
			diagnosis: "f-diagnosis",
		},
		onRender: forceRepaint,
		extraQuery: function () {
			return { patient_id: visitsPatientId };
		},
		cells: function (visit) {
			return [formatDate(visit.date.slice(0, 10)), visit.cause, visit.diagnosis];
		},
		url: function (visit) {
			return "visit.html?id=" + encodeURIComponent(visit.id);
		},
	});
}