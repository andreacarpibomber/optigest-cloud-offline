# OptiGest Cloud Offline

Versione locale di OptiGest Cloud che continua a funzionare quando il cloud non è raggiungibile (assenza di internet o cloud temporaneamente giù).

Il cloud resta la **sorgente di verità**. L'app offline è un "ponte": consente di consultare i dati già sincronizzati finché la connessione non torna.

## Funzionalità (fase 1, sola lettura)

- Login iniziale online, con lo stesso flusso delle app iOS/Android
- Download di pazienti e visite (tutti i dati testuali) in un DB locale protetto
- Ricerca e consultazione dei pazienti anche senza connessione
- Sync incrementale in background (pre-fetch iniziale + worker)
- Token scaduto: l'app continua a mostrare i dati già salvati

Fuori scope per ora: scrittura/modifica offline, GDPR avanzato, audit.

## Architettura

- **Launcher**: Rust (avvio e gestione dei processi)
- **Applicazione**: PHP
- **DB locale**: SQL, ~50-70 MB per studio, protetto
- **Worker**: processo in background per il sync incrementale
- **API di sync**: endpoint dedicati lato cloud (da realizzare)

```
Cloud OptiGest  <--(API sync, token)-->  Worker  -->  DB locale  -->  App PHP (UI offline)
                                                          ^
                                                   Launcher Rust
```

## Requisiti

- PHP 8.x
- Rust (toolchain stabile) per il launcher
- Accesso a internet al primo avvio e per i sync

## Installazione

L'installazione iniziale è gestita da EosMedTech.

## Avvio

```
# da completare
```

## Roadmap

- [ ] API di sync lato cloud
- [ ] Login e gestione token
- [ ] Download iniziale pazienti e visite
- [ ] Worker di sync incrementale
- [ ] UI di ricerca e consultazione pazienti
- [ ] Launcher Rust
- [ ] Cifratura DB locale
- [ ] Scrittura offline e risoluzione conflitti
- [ ] GDPR e audit

## Licenza

Proprietario. © EosMedTech Srl.


