# OptiGest Cloud Offline – Generazione di chiavi e certificati di licenza

Guida passo passo. Tutti i comandi si lanciano dalla cartella `app/scripts/` con **Python 3** (nessuna dipendenza da installare).

## 1. Come funziona (in 30 secondi)

| Elemento | Dove vive | Cosa fa |
|---|---|---|
| **Chiave master** (privata) | `secret/master.key` (mai nel repository né sul server) | Firma i certificati degli host. Non va mai su un server. |
| **Chiave master** (pubblica) | Incorporata nell'app da `build.rs`, ricavata da `secret/master.key` | Verifica che il certificato dell'host sia autentico. |
| **Chiave host** (privata) | `optigestcloudoffline.ini` sul server di quell'host | Firma la licenza (scadenza, organizzazione). |
| **Chiave host** (pubblica) | Dentro il certificato dell'host | Usata dall'app per verificare la firma della licenza. |
| **Certificato host** | `optigestcloudoffline.ini` sul server | `master` che dichiara: "questa chiave pubblica appartiene a questo host fino al giorno X". |

L'app scarica certificato e licenza dall'host a cui si connette e controlla: master → certificato → licenza. Chi non ha la master non può creare host o licenze valide.

## 2. Prerequisiti

- Python 3 (`python3 --version`)
- Gli script in `app/scripts/`: `_ed25519.py`, `gen_master_key.py`, `gen_license_keys.py`, `sign_host_key.py`
- Un posto sicuro **offline** per la master (chiavetta, gestore di password)

```bash
cd optigest-cloud-offline/app/scripts
```

## 3. Passo 1 – Chiave master (una sola volta)

```bash
python3 gen_master_key.py
```

- Crea `<repo>/secret/master.key` (cartella 700, file 600). Non sovrascrive file esistenti.
- Altro percorso: `python3 gen_master_key.py --out /percorso/master.key`
- `app/build.rs` legge `secret/master.key` a ogni compilazione e incorpora nell'app **solo la parte pubblica**. Non c'è nulla da incollare in `license.rs`. In alternativa (CI) si può impostare la variabile d'ambiente `OPTIGEST_MASTER_PUBLIC_KEY`.

**Cosa fare subito:**

1. `secret/` deve essere nel `.gitignore` del repository (`secret/`, `*.key`).
2. Copia `master.key` in un backup offline (chiavetta + gestore di password).
3. Ricompila l'app (`cargo tauri build` / `cargo tauri dev`). Se manca il file, la compilazione avvisa e la licenza risulta "non configurata".

> Se perdi la master non puoi più firmare nuovi host. Se te la rubano devi generarne una nuova **e rilasciare una nuova versione dell'app**.

## 4. Passo 2 – Coppia di chiavi dell'host (una per ogni host)

```bash
python3 gen_license_keys.py
```

Stampa due righe:

```
signing_key_private=...
signing_key_public=...
```

- La **privata** va nell'ini del server (Passo 4). Non condividerla, non incollarla in chat.
- La **pubblica** serve al Passo 3.

## 5. Passo 3 – Certificato dell'host (firmato dalla master)

```bash
python3 sign_host_key.py secret/master.key <host> '<chiave pubblica host>' <AAAA-MM-GG>
```

Esempio:

```bash
python3 sign_host_key.py secret/master.key www.optigest.app 'AbC...=' 2027-12-31
```

- `<host>` = esattamente l'host a cui l'app si collega (quello in configurazione dell'app, senza `https://` né path).
- `<AAAA-MM-GG>` = scadenza del certificato (`not_after`). Consiglio: 1–2 anni.
- Stampa `host_certificate=...`, da copiare nell'ini.

## 6. Passo 4 – File ini sul server

1. Parti da `cloud/optigestcloudoffline.ini.template`.
2. Salva come `optigestcloudoffline.ini` nella cartella indicata da `SYNC_SECRETS_FILE` (nel tuo caso `dashboard/co/`, tramite `dirname(__FILE__)."/optigestcloudoffline.ini"`).
3. Compila:

```ini
[license]
signing_key_private=<privata del Passo 2>
host_certificate=<certificato del Passo 3>
```

Regole:

- **Niente virgolette** attorno ai valori e nessuno spazio extra.
- Il file **non deve essere scaricabile dal web**. Verifica aprendo `https://<host>/dashboard/co/optigestcloudoffline.ini` nel browser: deve dare 403/404. Se lo scarica, blocca l'accesso (`.htaccess`: `Require all denied`) o spostalo fuori dalla document root.
- Permessi consigliati: `chmod 600`, proprietario = utente del web server.

## 7. Passo 5 – Scadenza della licenza

`getLicenseValidUntil()` (in `SyncAssistant.php`) legge la scadenza da `getCurrentLicense()`; se non disponibile usa `MAX(LicenseExpiresOn)`. La classe `WksAssistant` deve estendere `AppAssistant`. La data finisce nel payload firmato, quindi l'app non può estenderla da sola.

## 8. Passo 6 – Verifica

1. Apri l'app → accedi con il PIN.
2. **Impostazioni → Licenza → Ricontrolla**.
3. Stato atteso: **valida** con la data di scadenza corretta.

## 9. Manutenzione

| Evento | Azione |
|---|---|
| Nuovo host | Ripeti Passi 2, 3, 4 per quell'host (la master resta la stessa). |
| Certificato in scadenza | Rifirma con `sign_host_key.py` (stessa chiave host, nuova data) e aggiorna `host_certificate` nell'ini. Nessun rilascio dell'app. |
| Chiave host compromessa | Rigenera la coppia (Passo 2), rifirma (Passo 3), aggiorna l'ini. Nessun rilascio dell'app. |
| Master compromessa/persa | Nuova master (Passo 1), nuova versione dell'app, rifirma tutti gli host. |
| Chiave incollata in chat/email/repo | Considerala compromessa e rigenerala. |

## 10. Errori più comuni

| Messaggio | Causa | Soluzione |
|---|---|---|
| File dei segreti non trovato | Path sbagliato in `SYNC_SECRETS_FILE` | Controlla nome e cartella (`dashboard/co/`). |
| File dei segreti non leggibile | Permessi | `chmod 600` + proprietario corretto. |
| Chiave di firma non valida (attesi 64 byte in base64) | Chiave troncata, spazi o virgolette | Ricopia la riga `signing_key_private` per intero, senza virgolette. |
| Certificato host mancante/non valido | `host_certificate` vuoto o incompleto | Rigenera con `sign_host_key.py` e incolla tutta la riga. |
| Certificato non valido per questo host | Host nel certificato ≠ host dell'app | Rifirma con l'host esatto. |
| Certificato scaduto | `not_after` passato | Rifirma con nuova data (Sezione 9). |
| Chiave master non configurata | `MASTER_PUBLIC_KEY_B64` ancora placeholder | Manca `secret/master.key` in compilazione: Passo 1 e ricompila. |
| Firma non valida / licenza manomessa | Master dell'app ≠ master che ha firmato | Verifica che la pubblica nell'app corrisponda alla `master.key` usata. |

## 11. Regole di sicurezza

- `master.key`: solo offline, backup in due posti, mai in repo, server, chat o email.
- Chiave privata dell'host: solo nell'ini del suo server.
- Non salvare mai l'ini nel repository (aggiungi `optigestcloudoffline.ini` al `.gitignore`; nel repo resta solo il `.template`).
- `gen_license_keys.php` e `sign_host_key.php` in `cloud/` sono superati dalle versioni Python e si possono eliminare.