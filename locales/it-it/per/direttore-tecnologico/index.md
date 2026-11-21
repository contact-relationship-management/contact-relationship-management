# Per il direttore tecnologico

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Architettura

- **Servizio:** Rust su Axum e axa (sqlx 0.9), PostgreSQL 18, Redis per cache, contatori condivisi e propagazione del
  break-glass. `forbid(unsafe_code)`, clippy pedantic. La fonte di verità per il comportamento è la specifica
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Interfaccia:** SvelteKit 3 / Svelte 5, un proxy backend-for-frontend in modo che nessun token raggiunga mai il
  JavaScript del browser, CSP rigorosa con hash.
- **Autenticazione:** PASETO v4.public verificato offline, Keycloak facoltativo; controllo degli accessi basato sugli
  attributi (azioni di lettura/scrittura/eliminazione/distruttive, un obbligo `mask`, verifiche a livello di record)
  da un motore condiviso.
- **Audit:** catena di hash SHA-256 scritta nella stessa transazione della modifica, trigger in sola aggiunta,
  verificabile su `GET /api/audits/verify`.
- **Protezione dei dati:** cifratura dei campi AES-256-GCM con rotazione delle chiavi su due colonne; chiavi locali,
  nessun KMS.

## Prove di qualità

Test unitari (185), test sul database (56, tra cui applicazione dei controlli, mascheramento, audit, paginazione,
cache, idempotenza, verifiche dello schema), test unitari del front end (106) e Playwright (98), comprese le scansioni
di accessibilità axe. La CI esegue fmt, clippy, test, build e lint del front end e una fase sulla catena di fornitura
(`cargo audit`, `cargo deny`, `pnpm audit`, SBOM CycloneDX) su GitHub Actions e Woodpecker. Un solo comando la
riproduce: `scripts/ci-check.sh`.

## Debito tecnico e limiti onesti

- La paginazione usa un cursore a scostamento (le righe possono essere saltate o ripetute se i dati cambiano durante
  la scansione); nessun conteggio totale, per scelta.
- Le chiavi di idempotenza sono facoltative nell'API (T52): un client deve inviare `Idempotency-Key` in una creazione
  per essere protetto dal rischio che un nuovo tentativo duplichi un record. L'interfaccia inclusa ne invia una a ogni
  creazione.
- Gli schemi OpenAPI sono scritti a mano. Coprono richieste, elenchi, letture di dettaglio, dashboard, viste degli
  insight, audit, privacy e break-glass; le transizioni di stato e gli upsert che restituiscono il record aggiornato non
  sono tipizzati. Un test fallisce quando una route e il documento non concordano, e altri verificano rispetto agli
  schemi le risposte reali, comprese quelle mascherate
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Gli errori sono documenti problem RFC 9457 (T54) con i membri precedenti `error` e `description` mantenuti; gli errori
  di livelli esterni al router (limite di dimensione, timeout) non vengono convertiti. I criteri di versionamento e
  deprecazione sono in [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); le intestazioni `Deprecation` e
  `Sunset` sono specificate ma non realizzate.
- La cifratura dei campi copre due colonne; nessuna integrazione con un KMS.
- Lo stato di limitazione di frequenza e di cache richiede Redis per essere corretto con più repliche.
- Le viste degli insight sono mascherate per ruolo ma messe in cache per token del chiamante per 30 s.
- La riautenticazione di livello superiore prima delle azioni distruttive richiede lavoro sul fornitore di identità.

## Sequenza tecnica consigliata

1. Chiavi dei campi gestite da un KMS e copertura più ampia dei campi (resto di T41).
2. Creare una prima release con tag per provare la firma senza chiavi e la verifica (T49), e far controllare a un
   revisore la bozza degli standard API.
3. Realizzare le intestazioni `Deprecation` e `Sunset` con la prima deprecazione.
4. Commissionare il test indipendente (T42) su una build di staging.
