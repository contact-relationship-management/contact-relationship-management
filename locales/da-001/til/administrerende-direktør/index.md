# Til den administrerende direktør

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Beslutningen i ét afsnit

Dette er et CRM på demoniveau med et usædvanligt stærkt fundament for sikkerhed og privatliv. Det er **ikke klar** til at
rumme rigtige patient- eller borgerdata, og repositoriet siger det selv ([plan.md](../../../../spec/plan.md)). At tage
det i brug er en beslutning om at finansiere det resterende eksterne arbejde, ikke om at tænde for det.

## Hvad der findes

- Et fungerende CRM på tværs af salg, marketing, support og analyse med en Rust-API og en webbrugerflade, bygget efter
  en skrevet specifikation.
- Marketing med samtykke først, adgangskontrol, der skjuler følsomme værdier for roller, som ikke bør se dem, et
  revisionsspor, der afslører manipulation, indsigt og sletning til privatlivsanmodninger samt nødadgang
  ("break-glass"), som er begrundet, tidsbegrænset og gennemgået.
- 54 af 58 sporede arbejdspunkter er færdige ([tasks.md](../../../../spec/tasks.md)).

## Hvad der står mellem dette og rigtig brug

| Hindring | Hvorfor den har betydning | Hvem kan løse den |
|---|---|---|
| Ingen rigtig e-mailudsendelse (T39) | Marketing er simuleret; en rigtig udbyder kræver kontrakter og en databehandleraftale | Du vælger leverandøren; juridisk underskriver |
| Ingen uafhængig sikkerhedstest (T42) | Hospitaler og offentlige købere kræver som regel en; vores egne tests er ikke uafhængige | Et eksternt firma |
| Ingen manuel tilgængelighedsrevision (T48) | Offentlige organer skal offentliggøre en tilgængelighedserklæring, som de kan stå inde for | Personer med hjælpeteknologi |
| Udgivelsessignering er bygget, men har ikke kørt på en rigtig udgivelse, og ingen reviewer har gennemgået udkastet til API-standarder (T49) | Udbudstjeklister spørger om begge dele | Udviklingen (tag en første udgivelse); en ekstern reviewer |

## Beslutninger, der er dine

1. **Hvilket marked først.** Vurderingen efter NHS Digital Technology Assessment Criteria
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   konkluderer, at produktet sandsynligvis falder uden for DTAC's anvendelsesområde og ville dumpe de afsnit, der gælder,
   af mangel på organisatorisk dokumentation (en udpeget klinisk sikkerhedsansvarlig, certificeringer). Vælg vejen, før
   du bruger penge på den.
2. **Om uafhængig sikring skal finansieres** (T42, T48). Uden den er den ærlige påstand "bygget til høj standard, ikke
   testet af udenforstående".
3. **Hvem der ejer risikoen**, når rigtige data rummes. Se oversigterne for [risikodirektøren](../risikodirektør/index.md)
   og [juridisk direktør](../juridisk-direktør/index.md).

## Hvad man ikke skal sige udadtil

Påstå ikke overholdelse, certificering, "HIPAA-kompatibel" eller "godkendt af NHS". Dokumentationspakken
([compliance](../../../../spec/compliance/index.md)) understøtter en snævrere, sand udtalelse: kontroller er kortlagt,
implementerede er testet, huller er navngivet.
