// Schermata mostrata quando la licenza non consente l'uso dell'app (scaduta, da ricontrollare online, non valida).

async function showLicense() {
	const status = await invoke("get_license_status");
	if (!status.blocked) { location.replace("homepage.html"); return; }
	$("license-message").textContent = status.message;
	$("license-expiry").textContent = status.valid_until ? "Scadenza: " + formatDate(status.valid_until) : "";
}

async function recheck() {
	const button = $("btn-recheck");
	button.disabled = true;
	$("license-result").textContent = "Controllo in corso...";
	try {
		const status = await invoke("refresh_license");
		if (!status.blocked) { location.replace("homepage.html"); return; }
		$("license-message").textContent = status.message;
		$("license-result").textContent = "";
	} catch (error) {
		$("license-result").textContent = String(error);
	}
	button.disabled = false;
}

$("btn-recheck").addEventListener("click", recheck);
$("btn-logout").addEventListener("click", logout);
showLicense();