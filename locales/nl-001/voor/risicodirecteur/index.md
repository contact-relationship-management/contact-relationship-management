# Voor de risicodirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Risicopositie

**Over het geheel: niet klaar voor gereguleerde gegevens.** De software is voor haar
fase goed beheerst, maar de belangrijkste restrisico's worden niet gedekt door tests en processen
die alleen de organisatie die haar overneemt of externe partijen kunnen leveren.

## Belangrijkste risico's

| # | Risico | Grondslag van de beoordeling | Mitigatie in de repository | Restrisico |
|---|---|---|---|---|
| 1 | Onontdekte kwetsbaarheden | Geen onafhankelijke penetratietest (T42) | Eigen tests, poorten voor de toeleveringsketen, strikte headers/CSP | **Hoog** tot getest |
| 2 | Onthulling van gevoelige velden aan de verkeerde rol | Complexe toegangsregels | Maskering per rol op records, aggregaten en insights; een persona-matrixtest; een lek in accountdetails gevonden en verholpen (T47) | Middel: maskering is getest, niet onafhankelijk geverifieerd |
| 3 | Misbruik van noodtoegang door insiders | Break-glass verhoogt leesrechten | Schriftelijke reden, limiet van 60 minuten, alleen lezen, verplichte beoordeling door een tweede persoon, geaudit | Middel: er is geen alarm gekoppeld; niet in te trekken |
| 4 | Auditspoor gewijzigd | Vereist databasetoegang | Hashketen plus append-only trigger; meldt de eerste breuk | Laag-middel: de kophash moet buiten de database worden verankerd |
| 5 | Onrechtmatige marketing | Fouten in toestemming | Het verzendpad handhaaft toestemming, niet de UI; onveranderlijke toestemmingsgeschiedenis | Middel tot een echte provider is geïntegreerd en beoordeeld (T39) |
| 6 | Beschikbaarheid / misbruik | Openbare API | Snelheidsbeperking en blokkade, gedeeld via Redis | Middel: limieten vallen per replica terug als Redis uitvalt |
| 7 | Niet-naleving van toegankelijkheid | Plicht voor de publieke sector | Geautomatiseerde axe-scans; eerlijke toegankelijkheidsverklaring | **Niet gekwantificeerd** tot de handmatige audit (T48) |
| 8 | Toeleveringsketen | Afhankelijkheden en images | `cargo deny/audit`, `pnpm audit`, SBOM, digest-vastzettingen die Dependabot vernieuwt, een release-workflow met sleutelloze ondertekening | Middel: ondertekening is niet uitgevoerd op een echte release (T49) |
| 9 | Dubbele records door nieuwe pogingen | Idempotentiesleutels zijn optioneel in de API (T52) | Herhaling voor clients die de sleutel meesturen; de meegeleverde UI doet dat altijd | Laag |
| 10 | Verkeerde classificatie van gegevens | De uitrollende partij bepaalt wat een "contactpersoon" is | Scopenotitie en DPIA-sjabloon | Hangt van u af |

## Hiaten die van nature open blijven

T39 (echte e-mailprovider), T42 (onafhankelijke zekerheid), T48 (handmatige
toegankelijkheidsaudit), T49 (een eerste ondertekende release, een controle van het API-concept door een reviewer). Zie [tasks.md](../../../../spec/tasks.md)
en [plan.md](../../../../spec/plan.md).

## Voorgestelde risicoacties

Registreer risico's 1, 7 en 10 formeel; eis T42 vóór echte gegevens; wijs een
eigenaar aan voor de beoordeling van break-glass en het alarmeren; stel de frequentie voor het verankeren van de auditkop vast.
