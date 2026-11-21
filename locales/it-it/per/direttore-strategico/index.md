# Per il direttore strategico

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## La domanda strategica

A che cosa serve? Oggi è un **CRM di riferimento guidato dalla specifica** con uno strato di sicurezza, privacy e audit
insolitamente completo. Non è un prodotto con clienti e non compete con i CRM maturi
([COMPARISONS.md](../../../../COMPARISONS.md)). La scelta che avete davanti è se mantenerlo come riferimento o
investire per renderlo distribuibile in contesti regolamentati.

## Che cosa lo rende davvero diverso

| Punto di forza | Perché potrebbe contare | Quanto è solido |
|---|---|---|
| Specifica al primo posto: una sola specifica guida sia il servizio Rust sia l'interfaccia Svelte ([spec](../../../../spec/index.md)) | Più economico da verificare, portare e mantenere coerente | Reale; i test citano la specifica |
| Marketing basato sul consenso, applicato nel percorso di invio | Un requisito imprescindibile per acquirenti del settore sanitario e pubblico | Reale, ma non esiste ancora un mittente reale (T39) |
| Punteggio e previsione dei lead spiegabili, basati su regole | Acquirenti che devono giustificare decisioni riguardanti le persone | Reale; deliberatamente senza machine learning |
| Contatti ricevuti da un servizio di identità, non duplicati | Meno dati personali conservati, superficie di violazione più piccola | Reale; dipende dall'esistenza di quel servizio |
| Audit a prova di manomissione, mascheramento per ruolo, accesso di emergenza motivato | I controlli su cui gli acquirenti regolamentati chiedono per primi | Implementati e testati; non verificati in modo indipendente |

## Che cosa non lo rende diverso

L'ampiezza delle funzionalità. Preventivi, contratti, telefonia, punteggio con machine learning e pagamenti sono fuori
ambito ([scope](../../../../spec/scope.md)). Chi sceglie un CRM in base alle funzionalità ne sceglierà un altro.

## Opzioni

| Opzione | Che cosa significa | Costo e rischio | Prove necessarie |
|---|---|---|---|
| A. Restare un riferimento | Mantenerlo come libreria didattica e di modelli | Il più basso; il valore è reputazionale | Nessuna oltre a quelle di oggi |
| B. Nicchia nel settore regolamentato | Completare le voci aperte e venderlo o supportarlo per enti sanitari e pubblici | Il più alto; servono un fornitore, termini legali, test esterni, garanzia continua | Test indipendente (T42), audit di accessibilità (T48), email reale (T39) |
| C. Componenti, non prodotto | Offrire la catena di audit, il mascheramento e i modelli break-glass come parti riutilizzabili | Moderato; servono confezionamento e documentazione | Lavoro di estrazione; nessuna nuova certificazione |

**Raccomandazione: fare A ora e finanziare il test più economico di B prima di impegnarsi.** La valutazione del NHS ha
concluso che il prodotto probabilmente non rientra nell'ambito DTAC
([valutazione](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
quindi il percorso verso il mercato non è definito. Spendete prima per scoprire quale acquirente ne ha bisogno e che
cosa richiederebbe, non per certificazioni che nessuno ha chiesto.

## Piano di lavoro, nell'ordine suggerito dalle prove

1. Test di sicurezza indipendente e audit manuale di accessibilità (T42, T48): decidono se B sia possibile.
2. Fornitore di posta reale (T39) una volta scelti un fornitore e i termini sul trattamento dei dati; esistono il punto
   di innesto del mittente e un percorso SMTP, che funzionano con un server di prova locale, con link di annullamento
   dell'iscrizione firmato, elenco di soppressione e un webhook generico firmato per gli esiti, ma nessun fornitore è
   ancora integrato ([roadmap](../../../../spec/roadmap.md)).
3. Provare la firma delle release con una prima release con tag (T49): requisito minimo per gli appalti. Il flusso è
   realizzato; non è mai stato eseguito.
4. Solo dopo, le funzionalità di prodotto della roadmap (attribuzione, territori, orologi SLA in orario lavorativo,
   portale pubblico della knowledge base).

## Rischi strategici

- **Dichiarare più di quanto sia vero.** Il modo più rapido di perdere un acquirente regolamentato è una dichiarazione
  di conformità che le prove non sostengono. Vedere la scheda [GRC](../governance-rischio-conformità/index.md).
- **Manutenzione.** Il lavoro sulla sicurezza non finisce: aggiornamenti delle dipendenze, aggiornamento dei digest,
  rotazione delle chiavi, nuovi test. L'opzione B vi impegna a farlo.
- **Dipendenza da un servizio di identità** esterno a questo repository.
- **Un solo manutentore.** Il progetto ha un unico manutentore e dichiara che la profondità della revisione è quella di
  una sola persona ([MAINTAINERS.md](../../../../MAINTAINERS.md), [GOVERNANCE.md](../../../../GOVERNANCE.md)).
  L'opzione B richiede più di una persona in grado di rispondere del lavoro sulla sicurezza.

## Decisioni per voi

1. Quale tra A, B o C, e la data di riesame di tale scelta.
2. Chi è il primo acquirente di riferimento, individuato tramite conversazioni prima di spendere.
3. Se finanziare subito la garanzia indipendente (vedere la scheda dell'[amministratore delegato](../amministratore-delegato/index.md)).
