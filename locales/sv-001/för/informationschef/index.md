# För informationschef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Informations- och datavy

Det här CRM-systemet innehåller **relationsdata**: kontakter och konton som omslag kring identifierare som hålls i andra
system, aktiviteter, affärer, ärenden, samtyckeshistorik och en granskningslogg. Demografiska uppgifter stannar medvetet
i identitetsregistren ([domain-model](../../../../spec/domain-model.md)). Om din organisation registrerar patienter eller
medborgare som kontakter är klassificeringen av de uppgifterna **din** att avgöra;
planen är skriven efter den strängare läsningen ([plan](../../../../spec/plan.md)).

## Vart data tar vägen

| Lager | Innehåller | Skydd i arkivet |
|---|---|---|
| PostgreSQL | Alla poster, granskningskedjan, break-glass-beviljanden | Fältkryptering på två kolumner; utlösare för granskning som bara läggs till; kryptering i vila är driftsättarens |
| Redis | Cache för insiktssvar (30 s), hastighetsgränsräknare, nycklar för break-glass-beviljanden | Nycklade per anropare; **kräver TLS, ett lösenord och ingen offentlig exponering** |
| Loggar/mått | Begärandemetadata, Prometheus-mått | Bevarandet är driftsättarens |
| E-postleverantör (framtida) | Meddelandeinnehåll och mottagare | Inte byggd (T39) |

## Integration och identitet

- Inloggning kommer från en identitetstjänst som utfärdar PASETO-token, eller Keycloak.
  Anställning/förflyttning/avslut, MFA och sessionspolicy tillhör därför **din identitetsleverantör**; CRM-systemet
  tillämpar det token säger och vägrar utgångna.
- API:et är dokumenterat i OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  versionshanterat via header, sidindelat med `Link`-headers och returnerar fel som
  problemdokument enligt RFC 9457. Reglerna för ändring och utfasning finns i
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM:er (CycloneDX) tas fram för tjänsten och gränssnittet i CI; releaseflödet
  signerar dem och avbilderna.

## Informationsstyrningsansvar som kvarstår hos dig

Belägg för säkerhetskopiering och återställning, gallringsscheman per tabell, förankring av granskningskedjans
huvud utanför databasen, nyckelförvaring i KMS för fältnycklar, personuppgiftsbiträdesavtal för alla leverantörer
och konsekvensbedömningen avseende dataskydd
([dpia-template](../../../../spec/compliance/dpia-template.md)). Kontrollkartläggningen
([control-mapping](../../../../spec/compliance/control-mapping.md)) markerar varje kontroll som
Implemented, Deployer eller Gap.
