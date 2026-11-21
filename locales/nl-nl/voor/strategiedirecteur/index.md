# Voor de strategiedirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## De strategische vraag

Waar is dit voor? Vandaag is het een **CRM-referentie die de specificatie voorop stelt** met een
uitzonderlijk complete laag voor beveiliging, privacy en audit. Het is geen product met
klanten en het concurreert niet met volwassen CRM's
([COMPARISONS.md](../../../../COMPARISONS.md)). De keuze die voor u ligt is
of u het een referentie laat blijven, of investeert om het inzetbaar te maken in
gereguleerde omgevingen.

## Wat echt anders is

| Sterkte | Waarom het ertoe kan doen | Hoe solide het is |
|---|---|---|
| Specificatie eerst: één specificatie stuurt zowel de Rust-dienst als de Svelte-UI aan ([spec](../../../../spec/index.md)) | Goedkoper te auditen, over te zetten en consistent te houden | Echt; tests verwijzen naar de specificatie |
| Marketing waarbij toestemming voorop staat, afgedwongen in het verzendpad | Een harde eis voor kopers in de gezondheidszorg en de publieke sector | Echt, maar er bestaat nog geen echte verzender (T39) |
| Verklaarbare, regelgebaseerde leadscoring en prognoses | Kopers die beslissingen over mensen moeten kunnen verantwoorden | Echt; bewust geen machine learning |
| Contactpersonen worden uit een identiteitsdienst gehaald, niet gedupliceerd | Minder bewaarde persoonsgegevens, kleiner lekoppervlak | Echt; hangt af van het bestaan van die dienst |
| Audit die manipulatie zichtbaar maakt, maskering per rol, gemotiveerde noodtoegang | De maatregelen waar gereguleerde kopers het eerst naar vragen | Geïmplementeerd en getest; niet onafhankelijk geverifieerd |

## Wat niet anders is

Functionele breedte. Offertes, contracten, telefonie, scoring met machine learning en
betalingen vallen buiten de scope ([scope](../../../../spec/scope.md)). Wie een
CRM op functies kiest, kiest een ander.

## Opties

| Optie | Wat het betekent | Kosten en risico | Benodigd bewijs |
|---|---|---|---|
| A. Een referentie blijven | Houd het als leer- en patroonbibliotheek | Laagst; de waarde is reputatie | Niets meer dan vandaag |
| B. Niche in de gereguleerde sector | Maak de open punten af en verkoop of ondersteun het voor gezondheidszorg en overheidsinstanties | Hoogst; vereist een leverancier, juridische voorwaarden, externe tests, doorlopende zekerheid | Onafhankelijke test (T42), toegankelijkheidsaudit (T48), echte e-mail (T39) |
| C. Componenten, geen product | Bied de auditketen, maskering en break-glass-patronen aan als herbruikbare onderdelen | Gemiddeld; vereist verpakking en documentatie | Extractiewerk; geen nieuwe certificering |

**Aanbeveling: doe nu A en financier de goedkoopste test van B voordat u zich vastlegt.**
De NHS-beoordeling concludeerde dat het product waarschijnlijk buiten de scope van DTAC valt
([beoordeling](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
dus de route naar de markt is niet vastgesteld. Besteed eerst geld aan uitzoeken welke koper
dit nodig heeft en wat die zou eisen, niet aan certificeringen waar niemand om heeft gevraagd.

## Routekaart, in de volgorde die het bewijs suggereert

1. Onafhankelijke beveiligingstest en handmatige toegankelijkheidsaudit (T42, T48): zij
   bepalen of B echt mogelijk is.
2. Echte e-mailprovider (T39) zodra een leverancier en verwerkingsvoorwaarden
   zijn gekozen; de verzendnaad en een SMTP-pad bestaan en draaien tegen een lokale testserver, met een ondertekende afmeldlink, een onderdrukkingslijst en een generieke ondertekende uitkomstwebhook, maar er is nog geen provider geïntegreerd ([roadmap](../../../../spec/roadmap.md)).
3. Bewijs het ondertekenen van releases met een eerste getagde release (T49): een minimumeis voor
   inkoop. De workflow is gebouwd; hij is nog niet uitgevoerd.
4. Pas daarna de productfuncties op de routekaart (attributie, gebieden,
   SLA-klokken op basis van kantooruren, openbaar kennisbankportaal).

## Strategische risico's

- **Meer beweren dan waar is.** De snelste manier om een gereguleerde koper te verliezen is een
  nalevingsbewering die het bewijs niet ondersteunt. Zie het [GRC-overzicht](../governance-risico-compliance/index.md).
- **Onderhoud.** Beveiligingswerk is nooit af: updates van afhankelijkheden, vernieuwing van
  digests, sleutelrotatie, hertesten. Optie B bindt u eraan.
- **Afhankelijkheid van een identiteitsdienst** die buiten deze repository ligt.
- **Eén beheerder.** Het project heeft één beheerder en zegt dat de diepte van de
  review die van één persoon is ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Optie B vraagt meer dan één persoon
  die verantwoording kan afleggen over het beveiligingswerk.

## Beslissingen voor u

1. Welke van A, B of C, en de herzieningsdatum voor die keuze.
2. Wie de eerste doelkoper is, gevonden via gesprekken voordat er geld wordt uitgegeven.
3. Of onafhankelijke zekerheid nu wordt gefinancierd (zie het overzicht voor de [algemeen directeur](../algemeen-directeur/index.md)).
