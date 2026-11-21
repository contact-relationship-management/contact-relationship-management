# Governance, risiko, compliance

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Hvor man begynder

Dokumentationspakken er [spec/compliance](../../../../spec/compliance/index.md). Dens grundregel er den, holdet bør holdes
op på: en kontrol er **Implementeret**, kun når kode eller konfiguration håndhæver den, og en test eller kontrol fastholder
den; **Driftsansvarlig**, når din organisation selv skal levere den; **Hul**, når ingen af delene gælder i dag.

| Dokument | Brug det til |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 bilag A, GDPR art. 32, hver markeret som Implementeret, Driftsansvarlig eller Hul |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Konsekvensanalyse for databeskyttelse, forudfyldt med dette systems dataflow |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Hærdning, sikkerhedskopiering og gendannelse, nøgler, logning, hændelsesrespons |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Udkast til NHS DTAC-udsagn, som producenten kan overtage |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Selvgennemgang mod britiske statslige API-standarder (et udkast, ikke kontrolleret mod den offentliggjorte tekst) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Fejlformat (RFC 9457), hvad der tæller som en brydende ændring, trinene i udfasning og varslingsfristen |
| [NHS DTAC v2.0-vurdering](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Resultat spørgsmål for spørgsmål: sandsynligvis uden for DTAC's anvendelsesområde, og det ville dumpe de gældende afsnit af mangel på organisatorisk dokumentation |
| [plan](../../../../spec/plan.md) | Dom om parathed (ikke klar) og registeret over huller |
| [tasks](../../../../spec/tasks.md) | Hvert huls opgave, løsning og angivne grænser |

## Styring

- **Beslutningsret:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Regler for bidrag og AI-hjælp står i
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) og [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Ændringskontrol:** specifikationen er kilden til sandhed; kode og tests henviser til den. CI kører hos to udbydere,
  og `scripts/ci-check.sh` gengiver det. Regel i repositoriet: svæk ikke en test eller port for at få noget til at bestå.
- **Offentliggørelse:** [SECURITY.md](../../../../SECURITY.md) er vejen til at rapportere.

## Risiko

Risikobilledet står i [oversigten for risikodirektøren](../risikodirektør/index.md). Kort sagt: de største restrisici er
sikkerhed, der ikke er testet af udenforstående (T42), tilgængelighed, der ikke er opgjort (T48), og klassificering af
data, som er op til den, der tager systemet i brug.

## Compliance: hvad der er reelt, og hvad der ikke er

| Område | Stilling |
|---|---|
| Adgangskontrol | Implementeret og testet: rollebaseret maskering, kontrol på postniveau, driftslæsninger kun for administratorer, break-glass med obligatorisk gennemgang af en anden |
| Revision | Implementeret og testet: hashkædet, kun tilføjelse, i transaktionen, revision af læsninger inklusive lister, deallister og indsigt. **Driftsansvarlig:** forankr kædens hoved uden for databasen |
| Privatlivsrettigheder | Implementeret: eksport af indsigt, sletning (afvises, mens forpligtelser er åbne), oprydning efter opbevaring. **Driftsansvarlig:** retsgrundlag, opbevaringsplan, fortegnelse over behandling |
| Kryptering | Delvis: AES-256-GCM på feltniveau på to kolonner med lokale nøgler, ingen KMS. Kryptering i hvile og under transport er den driftsansvarliges |
| Forsyningskæde | Implementeret: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM'er, basisbilleder fastlåst med digest, Dependabot-opdateringer. **Hul:** workflowet for signerede udgivelser har ikke kørt på en rigtig udgivelse (T49) |
| Tilgængelighed | Delvis: automatiske axe-scanninger og en ærlig offentlig erklæring. **Hul:** ingen manuel revision (T48), oversættelser ugennemgåede |
| Uafhængig sikring | **Hul:** ingen (T42) |
| E-mail og lov om marketing | Udsendelsessti med samtykke først er implementeret; rigtig udsendelse findes kun som en SMTP-sti til udvikling; afmeldingslinket, spærrelisten og en signeret webhook for resultater findes, men ingen udbyder er integreret, og databehandleraftalen og den juridiske gennemgang er åbne (T39) |

## Kontroller, der har brug for en ejer i din organisation

1. Gennemgang og alarmering for break-glass-tildelinger (giv alarm ved `break_glass_granted`).
2. Forankring af revisionskædens hoved og verificering af det efter en plan (`GET /api/audits/verify`).
3. Dokumentation for sikkerhedskopiering og gendannelse samt prøve på nøglerotation.
4. Håndtering af ansættelse, flytning og fratrædelse i din identitetsudbyder.
5. Opbevaring af revisionstabellen, som vokser med læsetrafik.
6. Beskyttelse af Redis som databasen: den rummer cachelagret indsigtsdata.
7. Fletning af Dependabot-opdateringer og verificering af en signeret udgivelse, før den udrulles.

## Hvad man skal nægte at godkende

Ethvert udsagn om, at systemet er kompatibelt, certificeret, HIPAA-kompatibelt eller godkendt af NHS. Det forsvarlige
udsagn er snævrere: kontroller er kortlagt, de implementerede er testet, og hullerne er navngivet og sporet.
