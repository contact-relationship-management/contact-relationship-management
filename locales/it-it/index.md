# Contact Relationship Management

Software CRM open source per vendite, marketing e assistenza clienti.

Contact Relationship Management tiene traccia delle relazioni con clienti e potenziali clienti in vendite, marketing e assistenza. Conserva un'unica scheda per ogni contatto e richiede un consenso registrato prima dell'invio di qualsiasi messaggio. Il software non è pronto per la produzione: funziona con dati sintetici.

## Vantaggi

- **Scheda contatto unica.** Vendite, marketing, assistenza e analisi usano la stessa scheda.
- **Consenso registrato.** Nulla viene inviato a un contatto senza un consenso registrato. Ogni email include un link di annullamento dell'iscrizione firmato.
- **Meno dati personali.** Il CRM memorizza un riferimento a una persona in un servizio di identità, non una copia del nome o dell'indirizzo.
- **Risultati spiegabili.** I punteggi dei lead sono calcolati con regole leggibili. Le previsioni sono calcolate a partire dalle fasi della pipeline ponderate.
- **Progettazione internazionale.** Nomi, indirizzi, calendari e direzioni di scrittura non sono legati al formato di un solo paese. L'interfaccia è disponibile in otto lingue.
- **Specifica pubblica.** La specifica, il codice e i test sono pubblici. La documentazione indica che cosa è stato realizzato e che cosa no.

## Funzionalità

- **Vendite.** Punteggio dei lead basato su regole, pipeline delle trattative su una bacheca e previsioni ponderate per fase.
- **Marketing.** Campagne segmentate e sequenze di nurturing. L'invio è simulato, a meno che non sia configurata una modalità di posta.
- **Assistenza.** Ticket con priorità e scadenze di livello di servizio, e articoli della knowledge base.
- **Analisi.** Dashboard per tasso di successo, pipeline per fase, ritorno delle campagne, stato dei livelli di servizio e valore del cliente nel tempo.
- **Qualità dei dati.** Rilevamento dei duplicati per probabilità, con unione e annullamento.
- **Governance.** Registro di audit, accesso basato sui ruoli e supporto per le richieste di accesso ai dati e di cancellazione.

## Per iniziare

1. **Ottenere il codice.** Clonare il repository. Sono necessari Node.js e pnpm.
2. **Avviare l'applicazione.** Nella directory Svelte, eseguire `pnpm install`, poi `pnpm dev`.
3. **Accedere.** Usare `example@example.com`. Non è richiesto alcun altro servizio.
4. **Esaminare il tour.** Il tour copre contatti, lead, trattative, campagne, ticket e la vista direzionale, con dati fittizi.

Il [tutorial](../../docs/tutorial-first-tour.md) descrive ogni passaggio. [Installazione e uso](../../INSTALL.md) copre il servizio completo.

## Confronto

| Progetto | Categoria | Differenza |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM open source | Prodotto completo. Questo progetto è una dimostrazione più piccola, guidata dalla specifica. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM open source | Funzionalità più ampie. Questo progetto privilegia le regole spiegabili. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM open source | Maturo e di grandi dimensioni. Questo progetto è un riferimento compatto. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM commerciale SaaS | Questo progetto modella la stessa pratica. Non condivide codice né dati. |

Questo progetto è una dimostrazione e non compete con tali prodotti. Vedere [tutti i confronti](../../COMPARISONS.md).

## Schede per ruolo

Le [schede per ruolo](per/) offrono una pagina per ogni ruolo dirigenziale: che cosa fa il software, che cosa non fa e quali decisioni spettano a quel ruolo.

## Stato

Questo software non è pronto per la produzione ([percorso verso la produzione](percorso-verso-la-produzione/index.md)). Usare solo dati sintetici. Non viene inviata alcuna email reale, a meno che non sia configurata una modalità di posta reale. Nulla su questo sito costituisce consulenza legale.
