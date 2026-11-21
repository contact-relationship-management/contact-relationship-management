# Tester di sicurezza indipendente

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Perché esiste questa pagina

Ogni controllo di sicurezza di questo progetto è stato costruito e testato dai suoi autori (CRM-T42). Ciò dimostra che
i controlli funzionano come gli autori li hanno intesi. Non dimostra che non manchi nulla. Ospedali e acquirenti
governativi di norma richiedono un test da parte di un soggetto indipendente prima di conservare dati reali, e
l'affermazione onesta, fino ad allora, è «costruito secondo uno standard elevato, non verificato da esterni»
([scheda dell'amministratore delegato](../amministratore-delegato/index.md),
[scheda sulla sicurezza](../direttore-sicurezza/index.md)).

Questa pagina è ciò che si consegna a una società di test e ciò che ci si può aspettare in cambio.

## Che cosa significa superare il test, e che cosa no

Un rapporto pulito dice che un gruppo indicato, in un tempo indicato, con un ambito indicato, non ha trovato problemi
gravi. **Non** è una certificazione, non è un accreditamento e non è una garanzia. Non descrivete il sistema come
«sicuro», «certificato» o «sottoposto a penetration test secondo uno standard» oltre quanto afferma il rapporto
([scheda GRC](../governance-rischio-conformità/index.md)).

## Scegliere il tester

- Indipendente dagli autori e da chiunque abbia configurato la distribuzione.
- Composto da tester nominativi ed esperti, con l'accreditamento pertinente per il vostro mercato, dove ne esiste uno
  (per esempio CREST, o lo schema NCSC CHECK per i lavori per il governo britannico). Chiedete quale accreditamento
  copre le singole persone che svolgono il lavoro, non solo la società.
- Concorda per iscritto ambito, regole d'ingaggio, riservatezza, trattamento dei dati e responsabilità prima
  dell'inizio di qualsiasi test. Il contratto è di competenza dell'ufficio legale
  ([scheda del direttore degli affari legali](../direttore-affari-legali/index.md)).

## Che cosa testare

| Area | Che cosa dovrebbero provare i tester | Dove è descritta la progettazione |
|---|---|---|
| Autenticazione | Token falsificati, scaduti, riprodotti e con destinatario sbagliato; declassamento a non autenticato; il controllo all'avvio che rifiuta di funzionare con l'autenticazione disattivata | [auth](../../../../spec/auth.md) |
| Autorizzazione | Raggiungere record o azioni che un ruolo non dovrebbe: escalation orizzontale e verticale, verifiche a livello di record, azioni distruttive, letture riservate agli amministratori (audit, conservazione, revisione break-glass) | [auth](../../../../spec/auth.md) |
| Esposizione dei dati | Valori mascherati che trapelano da elenchi, letture di dettaglio aggregate, insight, aggregati, messaggi di errore, cache o chiave della cache | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Accesso di emergenza | Ottenere, prolungare o abusare del break-glass; riesaminare la propria concessione; riesaminare due volte | [auth](../../../../spec/auth.md) (T44) |
| Audit | Alterare, eliminare o riordinare la cronologia; aggirare la catena; nascondere le letture | [audit](../../../../spec/audit.md) |
| Resistenza agli abusi | Elusione di limite di frequenza e blocco, blocco di altri utenti, falsificazione delle intestazioni per la chiave del client | T43 |
| Gestione dell'input | Injection (SQL, JSON, intestazioni), corpi di dimensioni eccessive, cursori e chiavi di idempotenza malformati, riproduzione o collisione di chiavi tra chiamanti | T51, T52 |
| Front end web | Cross-site scripting, falsificazione delle richieste, gestione delle sessioni, il proxy backend-for-frontend, content security policy, clickjacking | UI `hooks.server.ts` |
| Infrastruttura così com'è distribuita | Immagini dei container, porte esposte, raggiungibilità di Redis e PostgreSQL, gestione dei segreti, valori predefiniti della configurazione | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Catena di fornitura | Provenienza di dipendenze e immagini, gli SBOM, build non firmate | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Logica di business | Elusione del consenso nel percorso di invio, cancellazione che lascia dati, manipolazione del punteggio, creazione di duplicati | [regulatory](../../../../spec/regulatory.md) |

**Fuori ambito, salvo diverso accordo:** il servizio di identità che emette i token (è esterno a questo repository), la
piattaforma di hosting e il denial of service su larga scala.

## Che cosa consegnare ai tester

1. Una **build di staging** che corrisponda a ciò che girerà in produzione: configurazione di produzione,
   autenticazione attiva, TLS, un vero Redis e un vero PostgreSQL. Non testate i valori predefiniti della demo.
2. **Solo dati sintetici.** L'attività di popolamento crea un portafoglio clienti senza persone reali. Dati reali di
   pazienti o cittadini non devono mai trovarsi in un ambiente di test.
3. **Un account per ruolo**, in modo da poter testare l'autorizzazione da ogni lato. Il criterio di riferimento è
   `config/abac-policy.reference.json`.
4. I documenti collegati sopra, il documento OpenAPI su `/api-docs/openapi.json` e l'accesso in lettura al repository,
   così che il test possa combinare black box e revisione del codice.
5. I **punti deboli noti**, in modo che i tester dedichino il tempo a ciò che non è già scritto:
   - la firma delle release è realizzata ma non è mai stata eseguita su una release reale (T49);
   - la cifratura dei campi copre due colonne con chiavi locali, nessun KMS;
   - la riautenticazione di livello superiore non è realizzata;
   - il break-glass non può essere revocato, solo scadere, e non è collegato alcun avviso;
   - i ricaricamenti dei criteri compaiono negli insight in cache per un massimo di 30 secondi;
   - la disconnessione per inattività avviene lato client;
   - lo stato di limitazione di frequenza ripiega per replica se Redis si guasta.
   Chiedete loro di confermare o confutare ciascuno, non di saltarli.

## Regole d'ingaggio da concordare per iscritto

- Tester nominativi, date e indirizzi di origine; un referente raggiungibile 24 ore su 24 dalla vostra parte.
- Che cosa si può tentare (per esempio, nessuna distruzione dell'archivio dati del test senza preavviso) e che cosa
  deve fermarsi alla prova dell'esposizione.
- Chi viene informato subito di un risultato critico, e come, prima del rapporto finale.
- Come vengono conservati e distrutti il rapporto, le prove e le eventuali note dei tester.

## Che cosa dovreste ricevere

- Un rapporto con ambito, metodo, date, nomi dei tester e accreditamento.
- Ogni risultanza con gravità, prove, passaggi per riprodurla, componente interessato e correzione consigliata.
- Un'indicazione chiara di che cosa **non** è stato testato.
- Un nuovo test delle risultanze corrette, incluso nell'incarico o quotato a parte.

## Dopo il rapporto

1. Classificare ogni risultanza in `spec/tasks.md` come attività con un responsabile e una data.
2. Correggere le risultanze critiche e alte prima di qualsiasi dato reale, poi far confermare le correzioni ai tester.
3. Aggiungere un test di regressione per ogni correzione. La regola del progetto è non indebolire un test o un
   controllo per farlo passare.
4. Registrare il risultato con onestà: la data del rapporto, l'ambito, le risultanze e il loro stato. Aggiornare il
   piano di prontezza ([plan](../../../../spec/plan.md)) e il pacchetto di conformità
   ([compliance](../../../../spec/compliance/index.md)).
5. Ripetere il test dopo modifiche sostanziali e secondo una cadenza stabilita dal vostro responsabile del rischio. Un
   test è un'istantanea, non uno stato.

## Ordine del lavoro

1. Chiudere prima ciò che è poco costoso e già noto, in modo che i tester non spendano il budget per riscoprirlo: una
   prima release firmata (T49), avvisi per il break-glass e per gli errori di verifica dell'audit, Redis dietro TLS e
   credenziali.
2. Preparare la build di staging, gli account e i dati di popolamento.
3. Concordare ambito, contratto e regole d'ingaggio.
4. Testare; ricevere subito le risultanze critiche.
5. Correggere, ripetere il test e registrare.
6. Solo allora aggiornare le dichiarazioni esterne, e solo entro ciò che il rapporto sostiene.

## Decisioni per voi

1. Chi commissiona e chi è responsabile delle risultanze (vedere la scheda del
   [direttore dei rischi](../direttore-rischi/index.md)).
2. Quale accreditamento il vostro mercato si aspetta dai tester.
3. Il budget e la data, stabiliti da ciò che costa un ritardo e non da ciò che è comodo.
4. Se testare una volta prima del lancio o con cadenza periodica.
