# Til it-direktøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Oversigt over information og data

Dette CRM rummer **relationsdata**: kontakter og konti som omslag om identifikatorer, der ligger i andre systemer,
aktiviteter, deals, sager, samtykkehistorik og en revisionslog. Demografiske oplysninger bliver bevidst i
identitetsregistrene ([domain-model](../../../../spec/domain-model.md)). Hvis din organisation registrerer patienter
eller borgere som kontakter, er det **dit** valg, hvordan disse data klassificeres; planen er skrevet efter den
strengere læsning ([plan](../../../../spec/plan.md)).

## Hvor data havner

| Lager | Rummer | Beskyttelse i repositoriet |
|---|---|---|
| PostgreSQL | Alle poster, revisionskæde, break-glass-tildelinger | Feltkryptering på to kolonner; revisionstrigger, der kun tilføjer; kryptering i hvile er den driftsansvarliges |
| Redis | Cache af indsigtssvar (30 s), tællere til hastighedsbegrænsning, nøgler til break-glass-tildelinger | Nøglet pr. kalder; **kræver TLS, en adgangskode og ingen offentlig eksponering** |
| Logge/målinger | Metadata om forespørgsler, Prometheus-målinger | Opbevaring er den driftsansvarliges |
| E-mailudbyder (fremtidig) | Beskedindhold og modtagere | Ikke bygget (T39) |

## Integration og identitet

- Indlogning kommer fra en identitetstjeneste, der udsteder PASETO-tokens, eller fra Keycloak. Ansættelse, flytning og
  fratrædelse, MFA og sessionspolitik er derfor **din identitetsudbyders**; CRM-systemet håndhæver, hvad tokenet siger,
  og afviser udløbne.
- API'en er dokumenteret i OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), versioneret med
  header, paginneret med `Link`-headere og returnerer fejl som RFC 9457-problemdokumenter. Reglerne for ændring og
  udfasning står i [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM'er (CycloneDX) fremstilles for tjeneste og brugerflade i CI; udgivelsesworkflowet signerer dem og billederne.

## Informationsstyring, der fortsat er dit ansvar

Dokumentation for sikkerhedskopiering og gendannelse, opbevaringsplaner pr. tabel, forankring af revisionskædens hoved
uden for databasen, KMS-opbevaring af feltnøgler, databehandleraftaler for enhver leverandør og
konsekvensanalysen for databeskyttelse ([dpia-template](../../../../spec/compliance/dpia-template.md)).
Kontrolkortlægningen ([control-mapping](../../../../spec/compliance/control-mapping.md)) markerer hver kontrol som
Implementeret, Driftsansvarlig eller Hul.
