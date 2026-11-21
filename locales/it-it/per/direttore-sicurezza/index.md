# Per il direttore della sicurezza

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## La postura di sicurezza in un paragrafo

Il servizio **fallisce in modo sicuro**, controlla l'accesso per ruolo e per record, nasconde i valori sensibili ai
ruoli che non devono vederli e mantiene una traccia di audit a prova di manomissione. Questi controlli sono
implementati e testati dagli autori. **Non sono stati testati da alcun soggetto indipendente** (T42), quindi
considerate ogni affermazione seguente come «realizzato e autoverificato», non «verificato».

## Controlli e dove verificarli

| Area | Che cosa c'è | Dove guardare |
|---|---|---|
| Autenticazione | PASETO v4.public verificato offline, Keycloak facoltativo; la produzione si rifiuta di avviarsi con l'autenticazione disattivata | [auth](../../../../spec/auth.md) |
| Autorizzazione | Basata sugli attributi: lettura, scrittura, eliminazione, azioni distruttive; un obbligo `mask`; verifiche a livello di record; audit, conservazione e revisione break-glass riservati agli amministratori | [auth](../../../../spec/auth.md), test a matrice di persone |
| Esposizione dei dati | Mascheramento su record, aggregati e insight; una fuga nel dettaglio dell'account è stata trovata e corretta (T47) | [tasks](../../../../spec/tasks.md) |
| Accesso di emergenza | Motivo scritto, limite di 60 minuti, sola lettura, riesame obbligatorio da una seconda persona, audit, condivisione tramite Redis | attività break-glass T44 |
| Audit | Catena di hash SHA-256 nella stessa transazione, trigger in sola aggiunta, endpoint di verifica; letture sottoposte ad audit | [audit](../../../../spec/audit.md) |
| Resistenza agli abusi | Limite di frequenza (600 al minuto) e blocco (10 errori in 5 minuti), condivisi tramite Redis; sonde esentate | T43 |
| Browser e trasporto | CSP rigorosa con hash, negazione dei frame, `nosniff`, referrer policy, CORS chiuso, intestazioni sicure; nessun token nel JavaScript del browser (BFF) | UI `hooks.server.ts` |
| Protezione dei dati | AES-256-GCM su due colonne con rotazione; chiavi locali, nessun KMS | T41 |
| Catena di fornitura | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, immagini rafforzate non root fissate per digest, aggiornamenti settimanali di Dependabot, un flusso di release che firma immagini e SBOM senza chiavi | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Errori | Ogni errore è un unico documento problem RFC 9457; un `401` indica lo schema accettato | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Sicurezza della memoria | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Punti deboli noti (dichiarati, non nascosti)

- **Nessun penetration test indipendente né esercizio di modellazione delle minacce** (T42). Il modello delle minacce in
  [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) è una sintesi, non un modello strutturato.
- **La firma delle release non è mai stata eseguita su una release reale** (T49). Il flusso e i comandi di verifica
  esistono; finché non viene inviato un tag non sono provati. Le action sono fissate per hash del commit.
- **La cifratura dei campi** copre due colonne con chiavi locali, nessun KMS.
- **La riautenticazione di livello superiore** prima delle azioni distruttive non è realizzata.
- **Redis contiene dati personali** (insight in cache, chiavi di concessione) e deve essere protetto come il database:
  TLS, password, nessuna esposizione pubblica.
- **Il break-glass** non può essere revocato, solo scadere, e non è collegato alcun avviso.
- **Limite di frequenza e blocco** ripiegano su conteggi per replica se Redis si guasta; il gruppo non autenticato è
  per indirizzo e, dietro un proxy non elencato in `TRUSTED_PROXIES`, è quello del proxy, quindi un aggressore può
  bloccare altri chiamanti non autenticati. Un chiamante con un token valido è contato per il suo soggetto.
- **I ricaricamenti dei criteri** compaiono negli insight in cache dopo fino a 30 secondi.
- **La disconnessione per inattività** avviene lato client.
- **Le chiavi di idempotenza** sono facoltative nell'API (T52): un client che non invia `Idempotency-Key` non è
  protetto dal rischio che una creazione ripetuta duplichi un record. L'interfaccia inclusa ne invia una.

## Che cosa deve fare chi distribuisce

1. Mantenere attiva l'autenticazione; fornire le chiavi di produzione e `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Generare un avviso su `break_glass_granted` e riesaminare tempestivamente le concessioni.
3. Ancorare la testa della catena di audit fuori dal database e verificarla (`GET /api/audits/verify`) con
   cadenza regolare.
4. Proteggere PostgreSQL e Redis (rete, TLS, credenziali, backup, cifratura a riposo); ruotare le chiavi dei campi con
   l'attività fornita.
5. Unire gli aggiornamenti di Dependabot; rieseguire la fase sulla catena di fornitura; verificare una release firmata
   prima di distribuirla.
6. Seguire il runbook per la risposta agli incidenti
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Gestione delle vulnerabilità

Le segnalazioni passano da [SECURITY.md](../../../../SECURITY.md). Commissionate il test indipendente (T42) su una
build di staging prima di qualsiasi dato reale e ripetetelo dopo ogni modifica sostanziale.

## Azioni di sicurezza suggerite, in ordine

1. Definire l'ambito e commissionare il penetration test indipendente (T42).
2. Inviare un primo tag di release per provare la firma e la verifica (T49).
3. Collegare gli avvisi per il break-glass e per gli errori di verifica della catena di audit.
4. Mettere Redis dietro TLS e credenziali in ogni ambiente.
5. Chiavi dei campi gestite da un KMS e copertura più ampia dei campi (resto di T41).

Vedere anche la scheda del [direttore tecnologico](../direttore-tecnologico/index.md) e quella del
[direttore dei rischi](../direttore-rischi/index.md).
