# Per il direttore degli affari legali

> Nota tecnica, non consulenza legale, certificazione o audit. Indica
> ciò che il repository mostra al 2026-10-11. Nulla qui afferma un accreditamento.

## La posizione legale in breve

Il repository **non è pronto per la produzione** e contiene dati sintetici. Non dichiara conformità ad alcun regime.
Implementa funzionalità tecniche che ne sostengono diversi ed elenca ciò che chi distribuisce deve aggiungere
([regulatory](../../../../spec/regulatory.md)).

## Regimi coinvolti

| Regime | Che cosa supporta il software | Che cosa dovete fornire |
|---|---|---|
| GDPR UE/Regno Unito | Cronologia dei consensi, esportazione per l'accesso ai dati, cancellazione (rifiutata finché esistono obblighi aperti), pulizia per conservazione, minimizzazione dei dati, audit delle letture | Base giuridica per ogni finalità, DPIA ([modello](../../../../spec/compliance/dpia-template.md)), registri dei trattamenti, piano di conservazione, ruoli di titolare/responsabile |
| PECR / e-privacy | Invii subordinati al consenso, annullamento immediato dell'iscrizione | Identificazione del mittente e decisioni sul soft opt-in per un mittente reale |
| HIPAA (se dati sanitari USA) | Controlli di audit, controllo degli accessi, integrità, funzionalità di sicurezza della trasmissione | Un accordo con il business associate, analisi dei rischi, politiche, formazione del personale. Il software non vi rende conformi |
| NHS DTAC / DCB0129/0160 | Bozze di prove ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Responsabile della sicurezza clinica designato, registro dei pericoli, certificazioni. La valutazione dice che DTAC probabilmente non si applica |
| Accessibilità (WCAG 2.2 AA, settore pubblico) | Controlli automatici e una dichiarazione onesta | Un audit manuale (T48) prima di qualsiasi dichiarazione di conformità |
| Sicurezza governativa (NIST 800-53, ISO 27001, Cyber Essentials) | Mappatura dei controlli ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certificazione e garanzia indipendente |

## Contratti che serviranno e che non esistono ancora

1. Accordo sul trattamento dei dati e condizioni per i trasferimenti internazionali con il fornitore di posta (T39).
2. Capitolato e condizioni di riservatezza con la società che esegue il penetration test (T42).
3. Condizioni di hosting e dei sub-responsabili per PostgreSQL, Redis e i log.
4. Licenza: vedere [LICENSE.md](../../../../LICENSE.md); il repository contiene anche dichiarazioni su sicurezza,
   governance e uso dell'IA.

## Punti di attenzione legale specifici

- **La cancellazione** anonimizza il contatto ed è rifiutata finché esiste una trattativa, un ticket o
  un'iscrizione attiva a una sequenza di nurturing; la catena di audit conserva le registrazioni delle azioni, quindi
  stabilite nella vostra DPIA come ciò interagisce con le richieste di cancellazione.
- **Le letture break-glass** di dati mascherati sono lecite solo sulla vostra base documentata; il campo del motivo è
  una prova, non un'autorità.
- **Il punteggio** dei lead: se mai influenzasse decisioni significative riguardanti le persone, si applicano le regole
  sulle decisioni automatizzate ([regulatory](../../../../spec/regulatory.md)).
- **Impegni sull'API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) prevede un preavviso di 180 giorni
  prima che una modifica dell'API venga rimossa. È una scelta del manutentore; se i clienti faranno affidamento su di
  essa, decidete se debba entrare in un contratto.
- **Dichiarazioni pubbliche:** non dichiarate conformità, certificazione o approvazione.
