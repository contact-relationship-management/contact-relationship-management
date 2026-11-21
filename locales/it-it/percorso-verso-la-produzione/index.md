# Percorso verso la produzione

> Contact Relationship Management non è pronto per la produzione. Questa pagina spiega che cosa significa qui
> produzione e a che punto è ciascuna parte.

Il software funziona con dati sintetici. L'email è simulata, a meno che non sia configurata una modalità di posta
reale. Non inserite dati personali reali.

## Che cosa significa produzione

Una distribuzione in produzione conserva i dati di persone reali e invia loro messaggi reali. Per questo progetto
occorre tutto quanto segue:

- **Accesso reale.** Ogni richiesta è autenticata e le regole di accesso corrispondono ai ruoli dell'organizzazione che
  effettua la distribuzione.
- **Invio lecito.** Ogni messaggio ha un consenso registrato e una base giuridica secondo la legge di ogni luogo a cui
  viene inviato, e passa da un vero fornitore di posta elettronica che gestisce rimbalzi, reclami e annullamenti
  dell'iscrizione.
- **Garanzia indipendente.** Persone esterne al progetto hanno verificato la sicurezza e l'accessibilità, e le
  risultanze sono corrette o accettate.
- **Operatività.** Backup di cui è stato provato il ripristino, monitoraggio, un processo per gli incidenti e qualcuno
  in reperibilità.
- **Prove di stabilità.** Esecuzioni lunghe con carico realistico, su hardware simile a quello della distribuzione,
  senza errori né rallentamenti.

La prontezza per la produzione si decide per ogni distribuzione. Chi distribuisce può chiudere alcuni controlli solo
per la propria organizzazione e i propri destinatari. La valutazione tecnica alla base di questa pagina è
[spec/plan.md](../../../spec/plan.md). I controlli aperti sono in [spec/tasks.md](../../../spec/tasks.md).

## Prontezza

Ogni percentuale è una stima fatta il 2026-11-13 a partire dall'elenco delle attività, non una misurazione.

- **Autenticazione e controllo degli accessi** (70%): Sono realizzati l'accesso con token PASETO o Keycloak/OIDC, le
  regole di accesso basate sugli attributi con verifiche a livello di record e una modalità di produzione che si
  rifiuta di avviarsi senza autenticazione. Nessuna distribuzione reale le ha attivate con i propri attributi, e la
  matrice di applicazione non copre ancora ogni handler (CRM-G1).
- **Recapito delle email** (40%): Il mittente predefinito è simulato ed esiste un mittente SMTP. Non c'è un adattatore
  per fornitori di posta con webhook per rimbalzi e reclami né un elenco di soppressione (CRM-T39).
- **Protezione dei dati e diritto dell'e-marketing** (15%): Sono realizzati registri del consenso, accesso ai dati,
  cancellazione e conservazione. Per questo progetto non è stata letta la legge di alcuna regione, e la revisione del
  percorso di invio reale è aperta (CRM-G2, [regulatory](../../../spec/regulatory.md)).
- **Garanzia indipendente sulla sicurezza** (10%): È scritto l'ambito di un penetration test. Non sono stati eseguiti
  né un penetration test, né un'analisi formale dei rischi, né un accreditamento (CRM-T42). Gli audit di sicurezza con
  IA non sono una garanzia indipendente.
- **Accessibilità** (50%): I controlli automatici axe-core rispetto a WCAG 2.2 A e AA passano su ogni route di pagina.
  Non è stato eseguito alcun audit manuale con uso della sola tastiera, screen reader o zoom al 400% (CRM-T48).
- **Traduzioni** (30%): L'interfaccia è disponibile in otto lingue. Le stringhe in lingue diverse dall'inglese sono
  scritte a macchina e nessun parlante le ha riviste.
- **Operatività** (30%): Un runbook operativo copre backup e ripristino, gestione delle chiavi, cifratura a riposo,
  risposta agli incidenti e conservazione dei log. Non è stato provato alcun ripristino e la checklist di messa in
  produzione del runbook è aperta.
- **Prestazioni e stabilità** (20%): Esistono test di carico e un test di stabilità di lunga durata. Il test di
  stabilità è durato due minuti sullo stack dimostrativo; non esistono né un'esecuzione lunga né una base di
  riferimento su hardware simile a quello di produzione.
- **Integrità delle release** (85%): Le immagini di release sono multi-architettura, firmate e attestate, e le loro
  firme risultano valide. Le action e le immagini sono fissate per digest e la cronologia è analizzata alla ricerca di
  segreti. La metà amd64 di un'immagine di release scaricata non è stata verificata e non esiste ancora un secondo host
  di codice (CRM-T192).
- **Integrazione continua** (100%): Uno script esegue ogni controllo su GitHub Actions e Woodpecker. Entrambe le basi di
  codice Rust e l'interfaccia hanno soglie minime del 95% delle righe; sono attivi controlli per browser, lint, catena di
  fornitura, mutazioni delle righe modificate e mutazioni complete settimanali (CRM-T94).
- **Salvaguardie della demo** (100%): Ogni visitatore del tour riceve una copia separata dei dati dimostrativi, limitata
  a 200 copie. Il tour è disattivato per impostazione predefinita; impostate
  `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` per offrirlo (CRM-T92).

## Come viene mantenuta questa pagina

Quando un controllo si chiude, modificate la stima e la relativa prova nello stesso commit del lavoro e collegate
l'attività.
