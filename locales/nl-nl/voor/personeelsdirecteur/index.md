# Voor de personeelsdirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Waarom mensen ertoe doen voor dit product

Toegang tot persoonsgegevens wordt bepaald door de rol. De software handhaaft wat het token van een
persoon zegt; **wie welke rol krijgt en wanneer die wordt ingetrokken, is een
mensenproces** dat deze repository niet voor u kan uitvoeren.

## Wat de software uw processen biedt

- **Zichtbaarheid per rol:** sommige rollen zien gemaskeerde waarden voor contactkanalen,
  dealbedragen en geldoverzichten; managers, beheerders en servicelagen zien meer
  ([auth](../../../../spec/auth.md)).
- **Geauditeerde leesacties:** het lezen van persoonsgegevens laat een spoor achter met de naam van de persoon.
  Medewerkers moeten dit vóór de livegang horen.
- **Noodtoegang:** een persoon kan een gemotiveerde leesverhoging van maximaal 60
  minuten nemen; een ander moet die achteraf beoordelen. Train beide kanten.
- **Uitloggen bij inactiviteit** na 15 minuten in de browser.
- **Toegankelijkheid:** een openbare verklaring en geautomatiseerde controles; de handmatige audit
  (T48) heeft nog gebruikers met een beperking en testers van hulptechnologie nodig.
- **Talen:** de UI is vertaald in 8 landinstellingen; de vertalingen zijn
  **niet** beoordeeld door moedertaalsprekers.

## Uw verantwoordelijkheden

| Gebied | Actie |
|---|---|
| Instroom, doorstroom, uitstroom | Ken rollen toe en trek ze in via uw identiteitsprovider; het CRM weigert verlopen tokens maar beheert geen accounts |
| Training | Gegevensbescherming, toestemmingsregels, gebruik en beoordeling van break-glass, toegankelijke werkwijze |
| Beleid | Aanvaardbaar gebruik, mededeling over monitoring (leesacties worden geaudit), tuchtroute bij misbruik van noodtoegang |
| Beoordelingscapaciteit | Benoem wie break-glass-toekenningen beoordeelt en wie vervangt bij afwezigheid |
| Werving | Rollen die het programma nodig heeft: contactpersoon voor de onafhankelijke tester, clinical safety officer bij NHS-scope, functionaris gegevensbescherming, toegankelijkheidsverantwoordelijke |
| Welzijn | Ondersteun medewerkers die gevoelige zaken behandelen; het CRM is geen systeem voor zaakbeheer maar kan gevoelige relatiegegevens bevatten |

## Openstaande punten waarvoor mensen nodig zijn

Handmatige toegankelijkheidsaudit met gebruikers van hulptechnologie (T48), beoordeling van de
vertalingen door moedertaalsprekers en het eigenaarschap van de hierboven genoemde beoordelings- en alarmeringstaken.
Zie [tasks.md](../../../../spec/tasks.md).
