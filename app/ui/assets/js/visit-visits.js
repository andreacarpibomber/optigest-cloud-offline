// Altre visite dello stesso paziente, nella scheda visita. La visita aperta è evidenziata; clic su una riga -> visit.html?id=...
// Parte da visit.js (showPatientVisits) quando la visita è caricata, perché serve l'ID del paziente.

function showPatientVisits(patientId, currentVisitId) {
	const table = new DataTable({
		autoStart: false,
		command: "search_visits",
		stateKey: "optigest.visit-page-visits." + patientId,
		defaultSort: "date",
		actions: false,
		filters: {
			date: "f-date",
			cause: "f-cause",
			diagnosis: "f-diagnosis",
		},
		extraQuery: function () {
			return { patient_id: patientId };
		},
		rowClass: function (visit) {
			return visit.id === currentVisitId ? "highlight-row" : "";
		},
		cells: function (visit) {
			return [formatDate(visit.date.slice(0, 10)), visit.cause, visit.diagnosis];
		},
		url: function (visit) {
			return "visit.html?id=" + encodeURIComponent(visit.id);
		},
		onRender: repaintVisitCards,
	});
	table.start();
}

// Nella WebView le card possono restare dipinte "a metà" quando l'altezza cambia dopo il caricamento dei dati:
// si nascondono e si rimostrano per forzare un ridisegno completo.
function repaintVisitCards() {
	requestAnimationFrame(function () {
		const cards = document.querySelectorAll(".detail-card, .visits-card");
		cards.forEach(function (card) { card.style.display = "none"; });
		void document.body.offsetHeight;
		cards.forEach(function (card) { card.style.display = ""; });
	});
}