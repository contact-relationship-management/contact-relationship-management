# Til risikodirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Risikobilledet

**Samlet: ikke klar til regulerede data.** Softwaren er godt kontrolleret for sit stadie, men de væsentligste
restrisici er ikke afbødet af test og proces, som kun den organisation, der tager den i brug, eller udenforstående kan
levere.

## Største risici

| # | Risiko | Grundlag for vurderingen | Afbødning i repositoriet | Rest |
|---|---|---|---|---|
| 1 | Uopdagede sårbarheder | Ingen uafhængig penetrationstest (T42) | Egne tests, porte for forsyningskæden, strenge headere/CSP | **Høj**, indtil den er testet |
| 2 | Afsløring af følsomme felter for den forkerte rolle | Komplekse adgangsregler | Rollebaseret maskering på poster, aggregater og indsigt; en persona-matrixtest; en lækage fundet og rettet i kontodetaljer (T47) | Middel: maskeringen er testet, ikke uafhængigt verificeret |
| 3 | Misbrug af nødadgang indefra | Break-glass forhøjer læsning | Skriftlig begrundelse, loft på 60 minutter, kun læsning, obligatorisk gennemgang af en anden, revideret | Middel: ingen alarm er koblet på; ingen tilbagekaldelse |
| 4 | Revisionsspor ændret | Kræver databaseadgang | Hashkæde plus trigger, der kun tilføjer; rapporterer det første brud | Lav-middel: hovedets hash skal forankres uden for databasen |
| 5 | Ulovlig marketing | Fejl i samtykke | Udsendelsesstien håndhæver samtykke, ikke brugerfladen; uforanderlig samtykkehistorik | Middel, indtil en rigtig udbyder er integreret og gennemgået (T39) |
| 6 | Tilgængelighed / misbrug | Offentlig API | Hastighedsgrænse og spærring, delt via Redis | Middel: grænserne falder tilbage til pr. replika, hvis Redis fejler |
| 7 | Manglende overholdelse af tilgængelighed | Pligt i den offentlige sektor | Automatiske axe-scanninger; ærlig tilgængelighedserklæring | **Ikke opgjort**, indtil den manuelle revision (T48) |
| 8 | Forsyningskæde | Afhængigheder og billeder | `cargo deny/audit`, `pnpm audit`, SBOM, digest-fastlåsninger opdateret af Dependabot, et nøglefrit signeret udgivelsesworkflow | Middel: signeringen har ikke kørt på en rigtig udgivelse (T49) |
| 9 | Dublerede poster ved genforsøg | Idempotensnøgler er valgfrie i API'en (T52) | Genafspilning for klienter, der sender nøglen; den medfølgende brugerflade gør det altid | Lav |
| 10 | Forkert klassificering af data | Den driftsansvarlige afgør, hvad en "kontakt" er | Note om anvendelsesområde og DPIA-skabelon | Afhænger af dig |

## Huller, der af natur forbliver åbne

T39 (rigtig e-mailudbyder), T42 (uafhængig sikring), T48 (manuel tilgængelighedsrevision), T49 (en første signeret
udgivelse, en reviewers kontrol af API-udkastet). Se [tasks.md](../../../../spec/tasks.md) og
[plan.md](../../../../spec/plan.md).

## Foreslåede risikohandlinger

Registrér risiko 1, 7 og 10 formelt; kræv T42 før rigtige data; udpeg en ejer af gennemgang og alarmering for
break-glass; fastsæt hyppigheden for forankring af revisionens hoved.
