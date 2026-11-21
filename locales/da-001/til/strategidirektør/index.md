# Til strategidirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Det strategiske spørgsmål

Hvad skal det bruges til? I dag er det et **CRM som reference, hvor specifikationen kommer først**, med et usædvanligt
komplet lag for sikkerhed, privatliv og revision. Det er ikke et produkt med kunder, og det konkurrerer ikke med modne
CRM-systemer ([COMPARISONS.md](../../../../COMPARISONS.md)). Valget foran dig er, om det skal forblive en reference,
eller om du vil investere i at gøre det anvendeligt i regulerede miljøer.

## Hvad der virkelig er anderledes

| Styrke | Hvorfor den kan have betydning | Hvor solid den er |
|---|---|---|
| Specifikationen først: én specifikation styrer både Rust-tjenesten og Svelte-brugerfladen ([spec](../../../../spec/index.md)) | Billigere at revidere, portere og holde konsistent | Reel; testene henviser til specifikationen |
| Marketing med samtykke først, håndhævet i udsendelsesstien | Et hårdt krav for købere i sundhed og den offentlige sektor | Reel, men der findes endnu ingen rigtig afsender (T39) |
| Forklarlig, regelbaseret lead-scoring og prognose | Købere, der skal begrunde afgørelser om personer | Reel; bevidst ingen maskinlæring |
| Kontakter hentes fra en identitetstjeneste, ikke duplikeret | Færre personoplysninger, mindre angrebsflade ved brud | Reel; afhænger af, at tjenesten findes |
| Manipulationssikker revision, maskering efter rolle, begrundet nødadgang | De kontroller, regulerede købere spørger om først | Implementeret og testet; ikke uafhængigt verificeret |

## Hvad der ikke er anderledes

Funktionsbredde. Tilbud, kontrakter, telefoni, scoring med maskinlæring og betalinger er uden for omfanget
([scope](../../../../spec/scope.md)). Enhver, der vælger CRM efter funktioner, vælger et andet.

## Muligheder

| Mulighed | Hvad den betyder | Pris og risiko | Nødvendigt bevis |
|---|---|---|---|
| A. Forbliv en reference | Behold det som et bibliotek til undervisning og mønstre | Lavest; værdien er omdømme | Intet ud over i dag |
| B. Niche i regulerede sektorer | Gør de åbne punkter færdige og sælg eller støt det til sundhedsvæsen og offentlige organer | Højest; kræver en leverandør, juridiske vilkår, ekstern test, løbende sikring | Uafhængig test (T42), tilgængelighedsrevision (T48), rigtig e-mail (T39) |
| C. Komponenter, ikke produkt | Tilbyd revisionskæden, maskeringen og break-glass-mønstrene som genbrugelige dele | Moderat; kræver pakning og dokumentation | Udtrækningsarbejde; ingen ny certificering |

**Anbefaling: gør A nu, og finansiér den billigste test af B, før du forpligter dig.** NHS-vurderingen konkluderede, at
produktet sandsynligvis falder uden for DTAC's anvendelsesområde
([vurdering](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
så vejen til markedet er ikke afklaret. Brug først penge på at finde ud af, hvilken køber der har brug for dette, og
hvad vedkommende ville kræve, ikke på certificeringer, ingen har bedt om.

## Køreplan i den rækkefølge, beviserne peger på

1. Uafhængig sikkerhedstest og manuel tilgængelighedsrevision (T42, T48): de afgør, om B overhovedet er muligt.
2. Rigtig e-mailudbyder (T39), når en leverandør og databehandlingsvilkår er valgt; afsendergrænsefladen og en
   SMTP-sti findes og kører mod en lokal testserver, med et signeret afmeldingslink, en spærreliste og en generisk
   signeret webhook for resultater, men ingen udbyder er endnu integreret ([roadmap](../../../../spec/roadmap.md)).
3. Bevis udgivelsessignering med en første mærket udgivelse (T49): en forudsætning for udbud. Workflowet er bygget; det
   har ikke kørt.
4. Først derefter produktfunktionerne på køreplanen (tilskrivning, territorier, SLA-ure efter åbningstider, offentligt
   portal til vidensbasen).

## Strategiske risici

- **At påstå mere, end der er sandt.** Den hurtigste måde at miste en reguleret køber på er en påstand om overholdelse,
  som beviserne ikke understøtter. Se [oversigten for GRC](../governance-risiko-compliance/index.md).
- **Vedligeholdelse.** Sikkerhedsarbejde bliver aldrig færdigt: opdatering af afhængigheder, fornyelse af digests,
  nøglerotation, ny test. Mulighed B forpligter dig til det.
- **Afhængighed af en identitetstjeneste**, der ligger uden for dette repositorium.
- **Én vedligeholder.** Projektet har én vedligeholder og siger, at gennemgangens dybde er én persons
  ([MAINTAINERS.md](../../../../MAINTAINERS.md), [GOVERNANCE.md](../../../../GOVERNANCE.md)). Mulighed B kræver mere end
  én person, der kan stå til ansvar for sikkerhedsarbejdet.

## Beslutninger til dig

1. Hvilken af A, B eller C, og datoen for at gennemgå det valg.
2. Hvem den første målgruppe af købere er, fundet gennem samtaler før pengene bruges.
3. Om uafhængig sikring skal finansieres nu (se [oversigten for den administrerende direktør](../administrerende-direktør/index.md)).
