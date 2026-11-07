#!/usr/bin/env python3
"""Genera la chiave di licenza (UNA volta sola, per tutti gli host). Solo Python 3, nessuna dipendenza.

    python3 gen_license_key.py

Salva la chiave privata in <repo>/secret/license.key (mai nel repository: fai un backup) e stampa:
  1) la riga da incollare in app/src/license.rs  (chiave PUBBLICA, nell'app)
  2) la riga da mettere in optigestcloudoffline.ini su ogni host  (chiave PRIVATA, solo sui server)
Non sovrascrive un file esistente: se esiste, ristampa solo le due righe.
"""

import base64
import os
import sys

import _ed25519

PATH = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "secret", "license.key"))


def main():
	if os.path.exists(PATH):
		private = base64.b64decode(open(PATH, encoding="utf-8").read().strip())
		if len(private) != 64: sys.exit("File chiave non valido: " + PATH)
		print("Uso la chiave esistente: " + PATH)
	else:
		private, _public = _ed25519.generate_keypair()
		os.makedirs(os.path.dirname(PATH), mode=0o700, exist_ok=True)
		descriptor = os.open(PATH, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
		with os.fdopen(descriptor, "w", encoding="utf-8") as handle: handle.write(base64.b64encode(private).decode() + "\n")
		print("Chiave creata: " + PATH + "  (fai un backup, non metterla nel repository)")
	print()
	print("1) In app/src/license.rs:")
	print('const LICENSE_PUBLIC_KEY_B64: &str = "' + base64.b64encode(private[32:]).decode() + '";')
	print()
	print("2) In optigestcloudoffline.ini, su ogni host:")
	print("[license]")
	print("signing_key_private=" + base64.b64encode(private).decode())


if __name__ == "__main__":
	main()