# Per il direttore dei rischi

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## Posizione di rischio

**Nel complesso: non pronto per dati regolamentati.** Il software è ben controllato per la sua fase, ma i principali
rischi residui non sono mitigati da test e processi che solo l'organizzazione che lo adotta o soggetti esterni possono
fornire.

## Rischi principali

| # | Rischio | Base della valutazione | Mitigazione nel repository | Residuo |
|---|---|---|---|---|
| 1 | Vulnerabilità non scoperte | Nessun penetration test indipendente (T42) | Test propri, controlli sulla catena di fornitura, intestazioni/CSP rigorose | **Alto** finché non testato |
| 2 | Divulgazione di campi sensibili al ruolo sbagliato | Regole di accesso complesse | Mascheramento per ruolo su record, aggregati e insight; un test a matrice di persone; una fuga trovata e corretta nel dettaglio dell'account (T47) | Medio: il mascheramento è testato, non verificato in modo indipendente |
| 3 | Uso improprio interno dell'accesso di emergenza | Il break-glass eleva le letture | Motivo scritto, limite di 60 minuti, sola lettura, riesame obbligatorio da una seconda persona, audit | Medio: nessun avviso collegato; nessuna revoca |
| 4 | Traccia di audit alterata | Richiede accesso al database | Catena di hash più trigger in sola aggiunta; segnala la prima rottura | Basso-Medio: l'hash della testa deve essere ancorato fuori dal DB |
| 5 | Marketing illecito | Errori sul consenso | Il percorso di invio applica il consenso, non l'interfaccia; cronologia dei consensi immutabile | Medio finché non è integrato e riesaminato un fornitore reale (T39) |
| 6 | Disponibilità / abuso | API pubblica | Limitazione di frequenza e blocco, condivisi tramite Redis | Medio: i limiti ripiegano per replica se Redis si guasta |
| 7 | Mancata conformità all'accessibilità | Obbligo del settore pubblico | Scansioni automatiche axe; dichiarazione di accessibilità onesta | **Non quantificato** finché non c'è l'audit manuale (T48) |
| 8 | Catena di fornitura | Dipendenze e immagini | `cargo deny/audit`, `pnpm audit`, SBOM, digest fissati e aggiornati da Dependabot, un flusso di release con firma senza chiavi | Medio: la firma non è mai stata eseguita su una release reale (T49) |
| 9 | Record duplicati dovuti a ripetizioni | Le chiavi di idempotenza sono facoltative nell'API (T52) | Riproduzione per i client che inviano la chiave; l'interfaccia inclusa lo fa sempre | Basso |
| 10 | Classificazione errata dei dati | Chi distribuisce decide che cos'è un «contatto» | Nota sull'ambito e modello di DPIA | Dipende da voi |

## Lacune che restano aperte per loro natura

T39 (fornitore di posta reale), T42 (garanzia indipendente), T48 (audit manuale di accessibilità), T49 (una prima
release firmata, un controllo della bozza dell'API da parte di un revisore). Vedere [tasks.md](../../../../spec/tasks.md)
e [plan.md](../../../../spec/plan.md).

## Azioni di rischio suggerite

Registrare formalmente i rischi 1, 7 e 10; richiedere T42 prima di qualsiasi dato reale; assegnare un responsabile per
il riesame e gli avvisi del break-glass; stabilire la cadenza dell'ancoraggio della testa dell'audit.
