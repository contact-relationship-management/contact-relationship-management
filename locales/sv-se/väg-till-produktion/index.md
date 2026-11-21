# Väg till produktion

> Contact Relationship Management är inte redo för produktion. Den här sidan beskriver vad produktion betyder här och
> hur långt varje del har kommit.

Programvaran körs på syntetiska data. E-post simuleras om inte ett riktigt e-postläge har konfigurerats. Lägg inte in
riktiga personuppgifter i den.

## Vad produktion betyder

En produktionsmiljö innehåller verkliga människors data och skickar verkliga meddelanden till dem. För det här projektet
krävs alla följande:

- **Riktig inloggning.** Varje begäran autentiseras, och åtkomstreglerna motsvarar den införande organisationens roller.
- **Lagligt utskick.** Varje meddelande har registrerat samtycke och en rättslig grund enligt lagen på varje plats dit det
  skickas, och går via en riktig e-postleverantör som hanterar studsar, klagomål och avregistreringar.
- **Oberoende säkerställande.** Personer utanför projektet har testat säkerheten och tillgängligheten, och
  resultaten är åtgärdade eller godtagna.
- **Drift.** Säkerhetskopior som har återställts, övervakning, en incidentprocess och någon i beredskap.
- **Belägg för stabilitet.** Långa körningar under realistisk belastning, på maskinvara som liknar driftsättningens, utan fel
  eller nedsatt prestanda.

Produktionsberedskap avgörs per driftsättning. En driftsättare kan stänga vissa grindar endast för sin egen organisation
och sina egna mottagare. Den tekniska bedömningen bakom den här sidan är [spec/plan.md](../../../spec/plan.md). De öppna
grindarna finns i [spec/tasks.md](../../../spec/tasks.md).

## Beredskap

Varje procentsats är en uppskattning gjord 2026-11-13 utifrån uppgiftslistan, inte en mätning.

- **Autentisering och åtkomstkontroll** (70%): Inloggning med PASETO-token eller Keycloak/OIDC, attributbaserade åtkomstregler
  med kontroller på postnivå och ett produktionsläge som vägrar starta utan autentisering är byggda. Ingen
  verklig driftsättning har slagit på dem med egna attribut, och tillämpningsmatrisen täcker ännu inte varje
  hanterare (CRM-G1).
- **E-postleverans** (40%): En simulerad avsändare är standard, och en SMTP-avsändare finns. Det finns ingen adapter för en
  e-postleverantör med webhooks för studsar och klagomål eller någon undertryckningslista (CRM-T39).
- **Dataskydd och lagstiftning om e-marknadsföring** (15%): Samtyckesposter, registerutdrag, radering och gallring är byggda.
  Ingen regions lagstiftning har lästs för det här projektet, och granskningen av den verkliga sändningsvägen är öppen (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Oberoende säkerhetssäkerställande** (10%): Ett omfång för penetrationstest är skrivet. Inget penetrationstest, ingen formell riskanalys
  och ingen ackreditering har genomförts (CRM-T42). AI-säkerhetsgranskningarna är inte oberoende säkerställande.
- **Tillgänglighet** (50%): Automatiska axe-core-kontroller mot WCAG 2.2 A och AA godkänns på varje sidväg. Ingen manuell
  granskning med enbart tangentbord, skärmläsare eller 400 % zoom har gjorts (CRM-T48).
- **Översättningar** (30%): Gränssnittet finns på åtta språk. Strängar på andra språk än engelska är
  maskinskrivna, och ingen språkbärare har granskat dem.
- **Drift** (30%): En driftsmanual täcker säkerhetskopiering och återställning, nyckelhantering, kryptering i vila,
  incidenthantering och loggbevarande. Ingen återställning har testats, och manualens checklista för driftsättning är öppen.
- **Prestanda och stabilitet** (20%): Belastningstester och ett långkörande stabilitetstest finns. Stabilitetstestet har
  körts i två minuter mot demostacken; ingen lång körning och ingen baslinje på produktionsliknande maskinvara finns.
- **Releaseintegritet** (85%): Releaseavbilder är flerarkitektur, signerade och attesterade, och deras signaturer
  verifieras. Actions och avbilder är låsta med digest, och historiken skannas efter hemligheter. Amd64-halvan av en hämtad
  releaseavbild har inte kontrollerats, och det finns ännu ingen andra kodvärd (CRM-T192).
- **Kontinuerlig integration** (100%): Ett skript kör alla kontroller på GitHub Actions och Woodpecker. Båda Rust-kodbaserna
  och gränssnittet har golv på 95 % radtäckning; grindar för webbläsare, lint, leveranskedja, mutationstest av ändrade rader
  och veckovis fullständigt mutationstest finns på plats (CRM-T94).
- **Demoskydd** (100%): Varje besökare på rundturen får en separat kopia av demodata, högst 200 kopior. Rundturen
  är avstängd som standard; sätt `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` för att erbjuda den (CRM-T92).

## Hur den här sidan hålls aktuell

När en grind stängs, ändra uppskattningen och dess belägg i samma incheckning som arbetet och länka uppgiften.
