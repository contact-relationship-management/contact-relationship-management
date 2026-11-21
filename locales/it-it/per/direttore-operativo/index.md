# Per il direttore operativo

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Che cosa serve per farlo funzionare

Il servizio è un processo Rust per ogni replica più PostgreSQL 18 e Redis; il front end è un processo Node. Entrambi
sono distribuiti come immagini di container rafforzate, non root, con immagini di base fissate per digest. Il runbook
operativo è [operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Fatti operativi da pianificare

- **Fallisce in modo sicuro all'avvio.** In produzione il servizio si rifiuta di avviarsi se l'autenticazione è
  disattivata, se la catena di audit non è utilizzabile o se mancano le chiavi richieste e
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`. Una distribuzione configurata male si ferma invece di funzionare aperta.
- **Salute e metriche.** `/_health/live`, `/_health/ready` e `/_health/startup` non sono mai soggetti a limitazione di
  frequenza, quindi un blocco non può riavviare un pod sano. Metriche su `/metrics.prom`.
- **I limiti contro gli abusi** sono attivi in produzione: 600 richieste al minuto per client e un blocco dopo 10
  accessi falliti in 5 minuti. Sono condivisi tra le repliche tramite Redis; se Redis si guasta, ripiegano su conteggi
  per replica.
- **Accesso di emergenza** (break-glass): qualsiasi utente connesso può ottenere un'elevazione motivata in sola lettura
  fino a 60 minuti. Qualcun altro deve riesaminare ogni concessione. **Processo che dovete eseguire:** generare un
  avviso sulle righe di audit `break_glass_granted` e controllare il numero di quelle non riesaminate (pagina di
  amministrazione `/break-glass/review`).
- **Gli errori** hanno un'unica forma (RFC 9457 `application/problem+json`, con `error` e `description` mantenuti),
  quindi l'analisi dei log e le regole di avviso possono basarsi su `status` e `type`.
- **Crescita dell'audit.** Le letture sono sottoposte ad audit, quindi la tabella di audit cresce con il traffico.
  Dimensionate la conservazione di conseguenza. Un client che esegue polling la farà crescere rapidamente.

## Lavoro operativo ancora aperto

| Voce | Stato |
|---|---|
| Esercitazioni di backup e ripristino, prova di rotazione delle chiavi | Descritte nel runbook; **voi** dovete eseguirle e documentarle |
| Ancoraggio della testa della catena di audit fuori dal database | Compito vostro; l'API riporta l'hash della testa |
| Aggiornamento dei digest delle immagini fissati | Dependabot apre pull request settimanali; **voi** dovete esaminarle e unirle, altrimenti i riferimenti diventano obsoleti |
| Verifica di una release firmata prima della distribuzione | I comandi sono nel runbook; distribuite per digest e rifiutate un'immagine che non supera la verifica. Non provato fino alla prima release con tag |
| Fornitore di posta reale (T39) | Aperto; cambia il quadro degli incidenti e dei fornitori |
| Redis contiene dati degli insight | Proteggetelo come il database: TLS, password, nessuna esposizione pubblica |

## Limiti noti da comunicare al team di reperibilità

La disconnessione per inattività avviene lato client; un token revocato viene rifiutato subito, ma un ricaricamento
dei criteri compare negli insight in cache dopo fino a 30 secondi; il break-glass non può essere revocato, solo
scadere.
