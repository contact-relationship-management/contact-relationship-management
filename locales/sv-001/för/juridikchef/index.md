# För juridikchef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Juridisk hållning i korthet

Arkivet är **inte redo för produktion** och innehåller syntetiska data. Det gör inte anspråk på
efterlevnad av något regelverk. Det implementerar tekniska funktioner som stöder
flera, och förtecknar vad en driftsättare måste lägga till ([regulatory](../../../../spec/regulatory.md)).

## Berörda regelverk

| Regelverk | Vad programvaran stöder | Vad du måste tillhandahålla |
|---|---|---|
| GDPR i Storbritannien/EU | Samtyckeshistorik, export av registerutdrag, radering (nekas så länge öppna förpliktelser finns), gallring, uppgiftsminimering, granskning av läsningar | Rättslig grund per ändamål, konsekvensbedömning (DPIA) ([mall](../../../../spec/compliance/dpia-template.md)), register över behandling, gallringsschema, roller som personuppgiftsansvarig/biträde |
| PECR / e-privacy | Utskick styrda av samtycke, omedelbar avregistrering | Avsändaridentifiering och beslut om soft opt-in för en riktig avsändare |
| HIPAA (om amerikanska hälsodata) | Granskningskontroller, åtkomstkontroll, integritet, funktioner för överföringssäkerhet | Ett biträdesavtal (business associate agreement), riskanalys, policyer, utbildning av personal. Programvaran gör inte dig regelefterlevande |
| NHS DTAC / DCB0129/0160 | Utkast till belägg ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Utsedd klinisk säkerhetsansvarig, riskförteckning, certifieringar. Bedömningen säger att DTAC sannolikt inte gäller |
| Tillgänglighet (WCAG 2.2 AA, offentlig sektor) | Automatiska kontroller och en ärlig redogörelse | En manuell granskning (T48) före varje påstående om efterlevnad |
| Statlig säkerhet (NIST 800-53, ISO 27001, Cyber Essentials) | Kontrollkartläggning ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certifiering och oberoende säkerställande |

## Avtal du kommer att behöva som ännu inte finns

1. Personuppgiftsbiträdesavtal och villkor för internationell överföring med e-postleverantören (T39).
2. Arbetsbeskrivning och sekretessvillkor med företaget som utför penetrationstestet (T42).
3. Hosting- och underbiträdesvillkor för PostgreSQL, Redis och loggar.
4. Licens: se [LICENSE.md](../../../../LICENSE.md); arkivet innehåller också
   redogörelser om säkerhet, styrning och användning av AI.

## Särskilda juridiska punkter att bevaka

- **Radering** anonymiserar kontakten och nekas så länge en öppen affär, ett öppet ärende
  eller en aktiv nurture-registrering finns; granskningskedjan behåller register över åtgärder,
  så avgör i din DPIA hur det samspelar med raderingsbegäranden.
- **Break-glass**-läsningar av maskerade data är lagliga endast på din dokumenterade grund;
  fältet för skäl är belägg, inte befogenhet.
- **Poängsättning** av leads: om den någonsin påverkar betydande beslut om
  människor gäller reglerna om automatiserat beslutsfattande ([regulatory](../../../../spec/regulatory.md)).
- **API-åtaganden:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  anger en uppsägningstid på 180 dagar innan en API-ändring tas bort. Det är
  underhållarens val; om kunder kommer att förlita sig på det, avgör om det hör hemma
  i ett avtal.
- **Offentliga påståenden:** påstå inte efterlevnad, certifiering eller godkännande.
