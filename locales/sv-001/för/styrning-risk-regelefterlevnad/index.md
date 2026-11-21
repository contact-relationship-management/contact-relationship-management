# Styrning, risk, regelefterlevnad

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Var du börjar

Bevispaketet är [spec/compliance](../../../../spec/compliance/index.md). Dess
grundregel är den som teamet ska hållas till: en kontroll är **Implemented** endast
när kod eller konfiguration tillämpar den och ett test eller en kontroll fastställer den; **Deployer**
när din organisation måste tillhandahålla den; **Gap** när ingetdera stämmer idag.

| Dokument | Använd det till |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR art. 32, var och en markerad Implemented, Deployer eller Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Konsekvensbedömning avseende dataskydd, förifylld med det här systemets dataflöden |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Härdning, säkerhetskopiering och återställning, nycklar, loggning, incidenthantering |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Utkast till NHS DTAC-redogörelser som tillverkaren kan anta |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Självgranskning mot brittiska statliga API-standarder (ett utkast, inte kontrollerat mot den publicerade texten) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Felformat (RFC 9457), vad som räknas som en brytande ändring, stegen för utfasning och uppsägningstiden |
| [Bedömning av NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Resultat fråga för fråga: sannolikt utanför DTAC:s omfång, och det skulle underkännas i de tillämpliga avsnitten på grund av bristande organisatoriska belägg |
| [plan](../../../../spec/plan.md) | Beredskapsutlåtande (inte redo) och luckregister |
| [tasks](../../../../spec/tasks.md) | Varje lucka, dess uppgift, lösning och angivna begränsningar |

## Styrning

- **Beslutsrätt:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Regler för bidrag och AI-assistans finns i
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) och [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Ändringskontroll:** specifikationen är sanningskällan; kod och tester
  hänvisar till den. CI körs hos två leverantörer, och `scripts/ci-check.sh` återskapar den.
  Regel i arkivet: försvaga inte ett test eller en grind för att få något att gå igenom.
- **Rapportering av sårbarheter:** [SECURITY.md](../../../../SECURITY.md) är rapporteringsvägen.

## Risk

Riskvyn finns i [riskchefens sammanfattning](../riskchef/index.md). Kort sagt: de högsta
kvarvarande riskerna är säkerhet som inte testats av utomstående (T42), okvantifierad
tillgänglighet (T48) och dataklassificering, som är införarens beslut.

## Regelefterlevnad: vad som är verkligt och vad som inte är det

| Område | Läge |
|---|---|
| Åtkomstkontroll | Implementerad och testad: rollbaserad maskering, kontroller på postnivå, driftläsningar endast för admin, break-glass med obligatorisk granskning av en andra person |
| Granskning | Implementerad och testad: hashkedjad, bara tillägg, i transaktion, granskning av läsningar inklusive listor, affärslistor och insikter. **Driftsättaren:** förankra kedjans huvud utanför databasen |
| Integritetsrättigheter | Implementerade: export av registerutdrag, radering (nekas så länge förpliktelser är öppna), gallring. **Driftsättaren:** rättslig grund, gallringsschema, register över behandling |
| Kryptering | Delvis: AES-256-GCM på fältnivå på två kolumner med lokala nycklar, ingen KMS. Kryptering i vila och under överföring är driftsättarens |
| Leveranskedja | Implementerad: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM:er, basavbilder låsta med digest, Dependabot-uppdateringar. **Lucka:** flödet för signerade releaser har inte körts på en riktig release (T49) |
| Tillgänglighet | Delvis: automatiska axe-skanningar och en ärlig offentlig redogörelse. **Lucka:** ingen manuell granskning (T48), översättningar ogranskade |
| Oberoende säkerställande | **Lucka:** ingen (T42) |
| E-post- och marknadsföringslag | Sändningsväg med samtycke först implementerad; riktigt utskick finns endast som en SMTP-väg för utveckling; avregistreringslänken, undertryckningslistan och en signerad webhook för utfall finns, men ingen leverantör är integrerad och biträdesavtalet och den juridiska granskningen är öppna (T39) |

## Kontroller som behöver en ägare i din organisation

1. Granskning och larm för break-glass-beviljanden (larma på `break_glass_granted`).
2. Förankring av granskningskedjans huvud och verifiering enligt schema
   (`GET /api/audits/verify`).
3. Belägg för säkerhetskopiering och återställning, samt repetition av nyckelrotation.
4. Hantering av anställning, förflyttning och avslut i din identitetsleverantör.
5. Gallring av granskningstabellen, som växer med läsningstrafiken.
6. Skydd av Redis som en databas: den innehåller cachade insiktsdata.
7. Sammanslagning av Dependabot-uppdateringar och verifiering av en signerad release före driftsättning.

## Vad du ska vägra godkänna

Varje påstående att systemet är regelefterlevande, certifierat, HIPAA-kompatibelt eller NHS-
godkänt. Det försvarbara påståendet är snävare: kontroller är kartlagda, de
implementerade är testade, och luckorna är namngivna och spårade.
