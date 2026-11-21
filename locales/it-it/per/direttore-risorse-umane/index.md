# Per il direttore delle risorse umane

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Perché le persone contano per questo prodotto

L'accesso ai dati personali è deciso dal ruolo. Il software applica ciò che dice il token di una persona; **a chi
viene assegnato quale ruolo, e quando viene tolto, è un processo delle persone** che questo repository non può svolgere
al posto vostro.

## Che cosa offre il software ai vostri processi

- **Visibilità basata sui ruoli:** alcuni ruoli vedono valori mascherati per i canali di contatto, gli importi delle
  trattative e le viste monetarie; i livelli di manager, amministratore e servizio ne vedono di più
  ([auth](../../../../spec/auth.md)).
- **Letture sottoposte ad audit:** leggere dati personali lascia una traccia che nomina la persona. Il personale
  dovrebbe essere informato prima della messa in produzione.
- **Accesso di emergenza:** una persona può ottenere un'elevazione motivata in lettura fino a 60 minuti; un'altra
  persona deve riesaminarla in seguito. Formate entrambe le parti.
- **Disconnessione per inattività** dopo 15 minuti nel browser.
- **Accessibilità:** una dichiarazione pubblica e controlli automatici; l'audit manuale (T48) richiede ancora utenti con
  disabilità e collaudatori di tecnologie assistive.
- **Lingue:** l'interfaccia è tradotta in 8 impostazioni locali; le traduzioni **non** sono state riviste da madrelingua.

## Le vostre responsabilità

| Area | Azione |
|---|---|
| Ingressi, trasferimenti, uscite | Assegnate e rimuovete i ruoli tramite il vostro fornitore di identità; il CRM rifiuta i token scaduti ma non gestisce gli account |
| Formazione | Protezione dei dati, regole sul consenso, uso e revisione del break-glass, pratica accessibile |
| Politiche | Uso accettabile, informativa sul monitoraggio (le letture sono sottoposte ad audit), percorso disciplinare per l'uso improprio dell'accesso di emergenza |
| Capacità di revisione | Indicate chi riesamina le concessioni break-glass e chi lo sostituisce quando è assente |
| Selezione del personale | Ruoli necessari al programma: referente per il collaudatore indipendente, responsabile della sicurezza clinica se rientra nell'ambito NHS, responsabile della protezione dei dati, responsabile dell'accessibilità |
| Benessere | Sostegno al personale che gestisce casi sensibili; il CRM non è un sistema di gestione dei casi ma può contenere dati di relazione sensibili |

## Voci aperte che richiedono persone

Audit manuale di accessibilità con utenti di tecnologie assistive (T48), revisione delle traduzioni da parte di
madrelingua e titolarità dei compiti di revisione e di avviso sopra indicati. Vedere [tasks.md](../../../../spec/tasks.md).
