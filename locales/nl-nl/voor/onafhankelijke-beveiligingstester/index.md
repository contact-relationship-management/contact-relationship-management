# Onafhankelijke beveiligingstester

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Waarom dit bestaat

Elke beveiligingsmaatregel in dit project is gebouwd en getest door de auteurs
(CRM-T42). Dat laat zien dat de maatregelen werken zoals de auteurs ze begrepen. Het laat
niet zien dat er niets ontbreekt. Ziekenhuizen en overheidskopers eisen meestal
een test door een onafhankelijke partij voordat ze echte gegevens bewaren, en de eerlijke bewering
tot dan is "gebouwd volgens een hoge standaard, niet getest door buitenstaanders"
([overzicht algemeen directeur](../algemeen-directeur/index.md),
[overzicht beveiliging](../beveiligingsdirecteur/index.md)).

Deze pagina is wat u aan een testbureau overhandigt, en wat u terug mag verwachten.

## Wat slagen betekent, en wat niet

Een schoon rapport zegt dat een benoemd team, in een benoemde tijd en met een benoemde scope, geen
ernstige problemen heeft gevonden. Het is **geen** certificering, geen accreditatie en geen
garantie. Beschrijf het systeem niet als "veilig", "gecertificeerd" of "penetratiegetest
volgens een standaard" verder dan het rapport vermeldt
([GRC-overzicht](../governance-risico-compliance/index.md)).

## De tester kiezen

- Onafhankelijk van de auteurs en van iedereen die de uitrol heeft geconfigureerd.
- Bemand met benoemde, ervaren testers, met relevante accreditatie voor uw
  markt waar die bestaat (bijvoorbeeld CREST, of het NCSC CHECK-schema voor
  Brits overheidswerk). Vraag welke accreditatie de personen dekt die het
  werk uitvoeren, niet alleen het bedrijf.
- Komt vóór het begin van enige test schriftelijk overeen over scope, spelregels, vertrouwelijkheid,
  omgang met gegevens en aansprakelijkheid. Het contract wordt door het
  juridische team afgesloten ([overzicht juridisch directeur](../juridisch-directeur/index.md)).

## Wat te testen

| Gebied | Wat de testers moeten proberen | Waar het ontwerp wordt beschreven |
|---|---|---|
| Authenticatie | Vervalste, verlopen, herhaalde tokens en tokens voor een verkeerde doelgroep; terugval naar niet-geauthenticeerd; de startcontrole die weigert te draaien zonder authenticatie | [auth](../../../../spec/auth.md) |
| Autorisatie | Records of acties bereiken die een rol niet hoort te bereiken: horizontale en verticale escalatie, controles op recordniveau, destructieve acties, leesacties alleen voor beheerders (audit, bewaring, break-glass-beoordeling) | [auth](../../../../spec/auth.md) |
| Blootstelling van gegevens | Gemaskeerde waarden die lekken via lijsten, gebundelde detailweergaven, insights, aggregaten, foutmeldingen, caching of de cachesleutel | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Noodtoegang | Break-glass nemen, verlengen of misbruiken; uw eigen toekenning beoordelen; tweemaal beoordelen | [auth](../../../../spec/auth.md) (T44) |
| Audit | Geschiedenis wijzigen, verwijderen of herordenen; de keten omzeilen; leesacties verbergen | [audit](../../../../spec/audit.md) |
| Weerstand tegen misbruik | Omzeilen van snelheidsbeperking en blokkade, andere gebruikers buitensluiten, headerspoofing voor de clientsleutel | T43 |
| Verwerking van invoer | Injectie (SQL, JSON, header), te grote berichttorso's, misvormde cursors en idempotentiesleutels, sleutels herhalen of laten botsen tussen aanroepers | T51, T52 |
| Web-front-end | Cross-site scripting, request forgery, sessiebeheer, de backend-for-frontend-proxy, content security policy, clickjacking | UI `hooks.server.ts` |
| Infrastructuur zoals geleverd | Containerimages, open poorten, bereikbaarheid van Redis en PostgreSQL, omgang met geheimen, standaardconfiguratie | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Toeleveringsketen | Herkomst van afhankelijkheden en images, de SBOM's, niet-ondertekende builds | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Bedrijfslogica | Toestemming omzeilen in het verzendpad, wissing die gegevens achterlaat, manipulatie van scores, aanmaak van duplicaten | [regulatory](../../../../spec/regulatory.md) |

**Buiten scope, tenzij u anders overeenkomt:** de identiteitsdienst die tokens
uitgeeft (die ligt buiten deze repository), het hostingplatform en denial-of-service
op volume.

## Wat de testers krijgen

1. Een **staging-build** die overeenkomt met wat in productie draait: productieconfiguratie,
   authenticatie aan, TLS, een echte Redis en PostgreSQL. Test niet
   de demostandaarden.
2. **Alleen synthetische gegevens.** De seedtaak bouwt een klantenbestand zonder echte
   mensen. Echte patiënten- of burgergegevens mogen nooit in een testomgeving staan.
3. **Eén account per rol**, zodat autorisatie vanaf elke kant kan worden getest. Het
   referentiebeleid is `config/abac-policy.reference.json`.
4. De hierboven gelinkte documenten, het OpenAPI-document op `/api-docs/openapi.json`
   en leestoegang tot de repository, zodat de test black-box en codereview kan
   combineren.
5. De **bekende zwakheden**, zodat testers hun tijd besteden aan wat nog niet is
   opgeschreven:
   - het ondertekenen van releases is gebouwd maar niet uitgevoerd op een echte release (T49);
   - veldversleuteling dekt twee kolommen met lokale sleutels, geen KMS;
   - stapsgewijze herauthenticatie is niet gebouwd;
   - break-glass kan niet worden ingetrokken, alleen verlopen, en er is geen alarm gekoppeld;
   - beleidsherladingen zijn tot 30 seconden zichtbaar in gecachete insights;
   - uitloggen bij inactiviteit gebeurt aan de clientkant;
   - de status van de snelheidsbeperking valt terug op per replica als Redis uitvalt.
   Vraag ze elk punt te bevestigen of te weerleggen, niet over te slaan.

## Spelregels die schriftelijk worden overeengekomen

- Benoemde testers, data en bronadressen; een contactpersoon die 24 uur bereikbaar is aan uw kant.
- Wat mag worden geprobeerd (bijvoorbeeld geen vernietiging van de testgegevensopslag
  zonder bericht) en wat moet stoppen bij bewijs van blootstelling.
- Wie direct wordt ingelicht over een kritieke bevinding, en hoe, vóór het eindrapport.
- Hoe het rapport, het bewijs en eventuele notities van testers worden opgeslagen en vernietigd.

## Wat u terug moet krijgen

- Een rapport met scope, methode, data, namen van testers en accreditatie.
- Elke bevinding met ernst, bewijs, stappen om te reproduceren, getroffen component
  en een aanbevolen oplossing.
- Een duidelijke verklaring van wat **niet** is getest.
- Een hertest van de opgeloste bevindingen, inbegrepen in de opdracht of geprijsd.

## Na het rapport

1. Triageer elke bevinding in `spec/tasks.md` als taak met een eigenaar en een datum.
2. Los kritieke en hoge bevindingen op vóór echte gegevens en laat de testers
   de oplossingen bevestigen.
3. Voeg voor elke oplossing een regressietest toe. De projectregel is geen test
   of poort te verzwakken om iets te laten slagen.
4. Leg het resultaat eerlijk vast: de rapportdatum, de scope, de bevindingen en
   hun status. Werk het gereedheidsplan ([plan](../../../../spec/plan.md)) en
   het compliancepakket ([compliance](../../../../spec/compliance/index.md)) bij.
5. Test opnieuw na een ingrijpende wijziging en volgens een schema dat uw risico-eigenaar vaststelt.
   Eén test is een momentopname, geen toestand.

## Volgorde van het werk

1. Sluit eerst wat goedkoop en bekend is, zodat de testers het budget niet besteden
   aan het opnieuw ontdekken ervan: een eerste ondertekende release (T49), alarmering op break-glass
   en op mislukte auditverificatie, Redis achter TLS en inloggegevens.
2. Bereid de staging-build, de accounts en de seedgegevens voor.
3. Kom de scope, het contract en de spelregels overeen.
4. Test; ontvang kritieke bevindingen direct.
5. Los op, test opnieuw en leg vast.
6. Werk pas daarna externe beweringen bij, en alleen tot wat het rapport ondersteunt.

## Beslissingen voor u

1. Wie opdracht geeft en wie de bevindingen beheert (zie het
   [overzicht risicodirecteur](../risicodirecteur/index.md)).
2. Welke accreditatie uw markt van de testers verwacht.
3. Het budget en de datum, bepaald door wat uitstel kost in plaats van door wat
   uitkomt.
4. Of u eenmalig vóór de lancering wilt testen of volgens een terugkerend schema.
