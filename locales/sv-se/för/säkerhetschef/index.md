# För säkerhetschef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Säkerhetsläget i ett stycke

Tjänsten **misslyckas säkert**, styr åtkomst per roll och per post, döljer
känsliga värden för roller som inte ska se dem och för ett manipuleringssäkert
granskningsspår. Dessa kontroller är implementerade och testade av upphovspersonerna. De har
**inte testats av någon oberoende part** (T42), så betrakta varje påstående nedan
som "byggt och självtestat", inte "verifierat".

## Kontroller och var du kontrollerar dem

| Område | Vad som finns | Var du tittar |
|---|---|---|
| Autentisering | PASETO v4.public verifieras offline, valfri Keycloak; produktion vägrar starta med autentisering avstängd | [auth](../../../../spec/auth.md) |
| Auktorisering | Attributbaserad: läsa, skriva, radera, destruktiv; ett `mask`-krav; kontroller på postnivå; granskning, gallring och break-glass-granskning endast för admin | [auth](../../../../spec/auth.md), persona-matristest |
| Dataexponering | Maskering av poster, aggregat och insikter; ett läckage i kontodetaljer hittades och åtgärdades (T47) | [tasks](../../../../spec/tasks.md) |
| Nödåtkomst | Skriftligt skäl, tak på 60 minuter, skrivskyddad, obligatorisk granskning av en andra person, granskad, delad via Redis | break-glass-uppgifter T44 |
| Granskning | SHA-256-hashkedja i samma transaktion, utlösare som bara läggs till, verifieringsslutpunkt; läsningar granskas | [audit](../../../../spec/audit.md) |
| Motståndskraft mot missbruk | Hastighetsgräns (600 per minut) och spärr (10 misslyckanden på 5 minuter), delade via Redis; prober undantagna | T43 |
| Webbläsare och transport | Strikt CSP med hashar, ramblockering, `nosniff`, referrer-policy, stängd CORS, säkra headers; inget token i webbläsarens JavaScript (BFF) | Gränssnittets `hooks.server.ts` |
| Dataskydd | AES-256-GCM på två kolumner med rotation; lokala nycklar, ingen KMS | T41 |
| Leveranskedja | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM:er, härdade avbilder utan root låsta med digest, veckovisa Dependabot-uppdateringar, ett releaseflöde som signerar avbilder och SBOM:er nyckellöst | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Fel | Varje fel är ett problemdokument enligt RFC 9457; ett `401` namnger det godtagna schemat | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Minnessäkerhet | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Kända svagheter (angivna, inte dolda)

- **Inget oberoende penetrationstest eller hotmodelleringsövning** (T42). Hotmodellen i
  [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  är en sammanfattning, inte en strukturerad.
- **Releasesignering har inte körts på en riktig release** (T49). Flödet och
  verifieringskommandona finns; tills en tagg pushas är de otestade. Actions
  är låsta med incheckningshash.
- **Fältkryptering** täcker två kolumner med lokala nycklar, ingen KMS.
- **Förnyad autentisering (step-up)** före destruktiva åtgärder är inte byggd.
- **Redis innehåller personuppgifter** (cachade insikter, beviljandenycklar) och måste skyddas
  som databasen: TLS, lösenord, ingen offentlig exponering.
- **Break-glass** kan inte återkallas, bara löpa ut, och inget larm är kopplat.
- **Hastighetsgräns och spärr** faller tillbaka på räkning per replika om Redis fallerar; den
  oautentiserade hinken är per adress, och bakom en proxy som inte finns i
  `TRUSTED_PROXIES` är det proxyns, så en angripare kan låsa ute andra
  oautentiserade anropare. En anropare med giltigt token räknas efter sitt subjekt.
- **Policyomladdningar** syns i cachade insikter i upp till 30 sekunder.
- **Utloggning vid inaktivitet** sker på klientsidan.
- **Idempotensnycklar** är frivilliga i API:et (T52): en klient som inte skickar
  `Idempotency-Key` är inte skyddad mot att ett omförsök vid skapande dubblerar en post.
  Det medföljande gränssnittet skickar en.

## Vad driftsättaren måste göra

1. Behåll autentiseringen på; ange produktionsnycklar och `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Larma på `break_glass_granted` och granska beviljanden skyndsamt.
3. Förankra granskningskedjans huvud utanför databasen och verifiera det
   (`GET /api/audits/verify`) enligt schema.
4. Skydda PostgreSQL och Redis (nätverk, TLS, autentiseringsuppgifter, säkerhetskopior, kryptering
   i vila); rotera fältnycklar med den medföljande uppgiften.
5. Slå ihop Dependabot-uppdateringarna; kör leveranskedjesteget igen; verifiera en signerad
   release före driftsättning.
6. Följ manualen för incidenthantering
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Hantering av sårbarheter

Rapporter går via [SECURITY.md](../../../../SECURITY.md). Beställ det oberoende
testet (T42) mot ett staging-bygge före alla verkliga data, och
testa om efter väsentlig förändring.

## Föreslagna säkerhetsåtgärder, i ordning

1. Avgränsa och beställ det oberoende penetrationstestet (T42).
2. Pusha en första releasetagg för att bevisa signering och verifiering (T49).
3. Koppla larm för break-glass och för misslyckad verifiering av granskningskedjan.
4. Placera Redis bakom TLS och autentiseringsuppgifter i varje miljö.
5. KMS-baserade fältnycklar och bredare fälttäckning (resten av T41).

Se även sammanfattningarna för [teknikchef](../teknikchef/index.md) och
[riskchef](../riskchef/index.md).
