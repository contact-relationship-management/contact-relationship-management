# Per l'amministratore delegato

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## La decisione in un paragrafo

Questo è un CRM di livello dimostrativo con una base di sicurezza e privacy insolitamente solida. **Non è pronto** per
conservare dati reali di pazienti o cittadini, e il repository lo dichiara da sé ([plan.md](../../../../spec/plan.md)).
Adottarlo significa decidere di finanziare il lavoro esterno ancora da fare, non di accenderlo.

## Che cosa esiste

- Un CRM funzionante per vendite, marketing, assistenza e analisi, con un'API Rust e un front end web, costruito su una
  specifica scritta.
- Marketing basato sul consenso, controllo degli accessi che nasconde i valori sensibili ai ruoli che non devono
  vederli, una traccia di audit che rivela le manomissioni, accesso ai dati e cancellazione per le richieste di privacy,
  e accesso di emergenza («break-glass») motivato, limitato nel tempo e riesaminato.
- 54 delle 58 attività monitorate sono completate ([tasks.md](../../../../spec/tasks.md)).

## Che cosa separa tutto questo dall'uso reale

| Ostacolo | Perché conta | Chi può risolverlo |
|---|---|---|
| Nessun recapito email reale (T39) | Il marketing è simulato; un vero fornitore richiede contratti e un accordo sul trattamento dei dati | Voi scegliete il fornitore; l'ufficio legale firma |
| Nessun test di sicurezza indipendente (T42) | Ospedali e acquirenti governativi di norma lo richiedono; i nostri test non sono indipendenti | Una società esterna |
| Nessun audit manuale di accessibilità (T48) | Gli enti pubblici devono pubblicare una dichiarazione di accessibilità che possano sostenere | Persone che usano tecnologie assistive |
| La firma delle release è realizzata ma non è mai stata eseguita su una release reale, e nessun revisore ha controllato la bozza degli standard API (T49) | Le liste di controllo degli appalti richiedono entrambe | Ingegneria (creare una prima release); un revisore esterno |

## Decisioni che spettano a voi

1. **Quale mercato per primo.** La valutazione dei Digital Technology Assessment Criteria del NHS
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   conclude che il prodotto probabilmente non rientra nell'ambito DTAC e non supererebbe le sezioni applicabili per
   mancanza di prove organizzative (responsabile della sicurezza clinica designato, certificazioni). Scegliete il
   percorso prima di spendere per esso.
2. **Se finanziare la garanzia indipendente** (T42, T48). Senza di essa l'affermazione onesta è «costruito secondo uno
   standard elevato, non verificato da esterni».
3. **Chi detiene il rischio** una volta conservati dati reali. Vedere le schede per il [direttore dei rischi](../direttore-rischi/index.md)
   e per il [direttore degli affari legali](../direttore-affari-legali/index.md).

## Che cosa non dire all'esterno

Non dichiarate conformità, certificazione, «conforme a HIPAA» o «approvato dal NHS». Il pacchetto di prove
([compliance](../../../../spec/compliance/index.md)) sostiene un'affermazione più ristretta e vera: i controlli sono
mappati, quelli implementati sono testati e le lacune sono indicate.
