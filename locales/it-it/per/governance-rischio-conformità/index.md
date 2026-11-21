# Governance, rischio, conformità

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Da dove cominciare

Il pacchetto di prove è [spec/compliance](../../../../spec/compliance/index.md). La sua regola di base è quella a cui
tenere il team: un controllo è **Implementato** solo quando codice o configurazione lo applicano e un test o una
verifica lo fissano; **A carico di chi distribuisce** quando deve fornirlo la vostra organizzazione; **Lacuna** quando
oggi non è vera nessuna delle due cose.

| Documento | A che cosa serve |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Allegato A, art. 32 GDPR, ciascuno contrassegnato come Implementato, A carico di chi distribuisce o Lacuna |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Valutazione d'impatto sulla protezione dei dati, precompilata con i flussi di dati di questo sistema |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Rafforzamento, backup e ripristino, chiavi, registrazione, risposta agli incidenti |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Bozze di dichiarazioni NHS DTAC che il produttore può adottare |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Autovalutazione rispetto agli standard API del governo britannico (una bozza, non confrontata con il testo pubblicato) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Formato degli errori (RFC 9457), che cosa costituisce una modifica incompatibile, i passaggi di deprecazione e il periodo di preavviso |
| [Valutazione NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Risultato domanda per domanda: probabilmente fuori dall'ambito DTAC, e non supererebbe le sezioni applicabili per mancanza di prove organizzative |
| [plan](../../../../spec/plan.md) | Giudizio di prontezza (non pronto) e registro delle lacune |
| [tasks](../../../../spec/tasks.md) | L'attività di ogni lacuna, la risoluzione e i limiti dichiarati |

## Governance

- **Diritti decisionali:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Le regole su contributi e assistenza dell'IA sono in
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) e [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Controllo delle modifiche:** la specifica è la fonte di verità; codice e test la citano. La CI viene eseguita su
  due fornitori e `scripts/ci-check.sh` la riproduce. Regola nel repository: non indebolire un test o un controllo per
  farlo passare.
- **Divulgazione:** [SECURITY.md](../../../../SECURITY.md) è il canale di segnalazione.

## Rischio

La visione del rischio è nella scheda del [direttore dei rischi](../direttore-rischi/index.md). In breve: i rischi
residui più alti sono la sicurezza non testata da esterni (T42), l'accessibilità non quantificata (T48) e la
classificazione dei dati, che spetta a chi adotta il sistema.

## Conformità: che cosa è reale e che cosa no

| Area | Posizione |
|---|---|
| Controllo degli accessi | Implementato e testato: mascheramento per ruolo, verifiche a livello di record, letture operative riservate agli amministratori, break-glass con riesame obbligatorio da una seconda persona |
| Audit | Implementato e testato: catena di hash, sola aggiunta, nella transazione, audit delle letture compresi elenchi, elenchi di trattative e insight. **A carico di chi distribuisce:** ancorare la testa della catena fuori dal database |
| Diritti sulla privacy | Implementati: esportazione per l'accesso ai dati, cancellazione (rifiutata finché esistono obblighi aperti), pulizia per conservazione. **A carico di chi distribuisce:** base giuridica, piano di conservazione, registri dei trattamenti |
| Cifratura | Parziale: AES-256-GCM a livello di campo su due colonne con chiavi locali, nessun KMS. La cifratura a riposo e in transito spetta a chi distribuisce |
| Catena di fornitura | Implementata: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, immagini di base fissate per digest, aggiornamenti di Dependabot. **Lacuna:** il flusso di release firmata non è mai stato eseguito su una release reale (T49) |
| Accessibilità | Parziale: scansioni automatiche axe e una dichiarazione pubblica onesta. **Lacuna:** nessun audit manuale (T48), traduzioni non riviste |
| Garanzia indipendente | **Lacuna:** nessuna (T42) |
| Posta elettronica e diritto del marketing | Percorso di invio basato sul consenso realizzato; l'invio reale esiste solo come percorso SMTP per lo sviluppo; esistono il link di annullamento dell'iscrizione, l'elenco di soppressione e un webhook firmato per gli esiti, ma nessun fornitore è integrato e l'accordo con il responsabile del trattamento e la revisione legale sono aperti (T39) |

## Controlli che necessitano di un responsabile nella vostra organizzazione

1. Riesame e avvisi per le concessioni break-glass (avviso su `break_glass_granted`).
2. Ancoraggio della testa della catena di audit e sua verifica con cadenza regolare (`GET /api/audits/verify`).
3. Prove di backup e ripristino e prova di rotazione delle chiavi.
4. Gestione di ingressi, trasferimenti e uscite nel vostro fornitore di identità.
5. Conservazione per la tabella di audit, che cresce con il traffico di lettura.
6. Protezione di Redis come il database: contiene dati degli insight in cache.
7. Unione degli aggiornamenti di Dependabot e verifica di una release firmata prima di distribuirla.

## Che cosa rifiutarsi di approvare

Qualsiasi dichiarazione che il sistema sia conforme, certificato, conforme a HIPAA o approvato dal NHS. L'affermazione
difendibile è più ristretta: i controlli sono mappati, quelli implementati sono testati e le lacune sono indicate e
monitorate.
