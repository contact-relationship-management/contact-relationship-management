# Per il direttore dei sistemi informativi

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Visione dell'informazione e dei dati

Questo CRM conserva **dati di relazione**: contatti e conti come involucri attorno a identificativi custoditi in altri
sistemi, attività, trattative, ticket, cronologia dei consensi e un registro di audit. I dati demografici restano
deliberatamente nei registri delle identità ([domain-model](../../../../spec/domain-model.md)). Se la vostra
organizzazione registra pazienti o cittadini come contatti, la classificazione di tali dati spetta **a voi**; il piano
è scritto secondo la lettura più rigorosa ([plan](../../../../spec/plan.md)).

## Dove vanno i dati

| Archivio | Contiene | Protezione nel repository |
|---|---|---|
| PostgreSQL | Tutti i record, la catena di audit, le concessioni break-glass | Cifratura dei campi su due colonne; trigger di audit in sola aggiunta; la cifratura a riposo spetta a chi distribuisce |
| Redis | Cache delle risposte degli insight (30 s), contatori di limitazione della frequenza, chiavi di concessione break-glass | Chiavi per chiamante; **richiede TLS, una password e nessuna esposizione pubblica** |
| Log/metriche | Metadati delle richieste, metriche Prometheus | La conservazione spetta a chi distribuisce |
| Fornitore di posta (futuro) | Contenuto dei messaggi e destinatari | Non realizzato (T39) |

## Integrazione e identità

- L'accesso proviene da un servizio di identità che emette token PASETO, oppure da Keycloak. Ingresso, trasferimento e
  uscita del personale, MFA e criteri di sessione spettano quindi **al vostro fornitore di identità**; il CRM applica
  ciò che dice il token e rifiuta quelli scaduti.
- L'API è documentata in OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), versionata tramite
  intestazione, paginata con intestazioni `Link` e restituisce gli errori come documenti problem RFC 9457. Le regole di
  modifica e deprecazione sono in [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- Gli SBOM (CycloneDX) sono prodotti per servizio e interfaccia nella CI; il flusso di release firma sia gli SBOM sia
  le immagini.

## Obblighi di governance dell'informazione che restano a voi

Prove di backup e ripristino, piani di conservazione per tabella, ancoraggio della testa della catena di audit fuori
dal database, custodia delle chiavi dei campi in un KMS, accordi sul trattamento dei dati per ogni fornitore e la
valutazione d'impatto sulla protezione dei dati ([dpia-template](../../../../spec/compliance/dpia-template.md)). La
mappatura dei controlli ([control-mapping](../../../../spec/compliance/control-mapping.md)) contrassegna ogni controllo
come Implementato, A carico di chi distribuisce o Lacuna.
