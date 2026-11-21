# Til den juridiske direktør

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Den juridiske stilling i korte træk

Repositoriet er **ikke klar til produktion** og rummer syntetiske data. Det gør ikke krav på overholdelse af nogen
ordning. Det implementerer tekniske funktioner, der understøtter flere, og opregner, hvad en driftsansvarlig selv skal
tilføje ([regulatory](../../../../spec/regulatory.md)).

## Berørte regelsæt

| Regelsæt | Hvad softwaren understøtter | Hvad du selv skal levere |
|---|---|---|
| GDPR (Storbritannien/EU) | Samtykkehistorik, eksport af indsigt, sletning (afvises, mens åbne forpligtelser findes), oprydning efter opbevaring, dataminimering, revision af læsninger | Retsgrundlag pr. formål, DPIA ([skabelon](../../../../spec/compliance/dpia-template.md)), fortegnelse over behandling, opbevaringsplan, roller som dataansvarlig/databehandler |
| PECR / e-privacy | Udsendelser betinget af samtykke, øjeblikkelig afmelding | Afsenderidentifikation og afgørelser om soft opt-in for en rigtig afsender |
| HIPAA (hvis amerikanske sundhedsdata) | Revisionskontroller, adgangskontrol, integritet, funktioner til sikker overførsel | En business associate-aftale, risikoanalyse, politikker, uddannelse af medarbejdere. Softwaren gør dig ikke kompatibel |
| NHS DTAC / DCB0129/0160 | Udkast til dokumentation ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Udpeget klinisk sikkerhedsansvarlig, fareregister, certificeringer. Vurderingen siger, at DTAC sandsynligvis ikke gælder |
| Tilgængelighed (WCAG 2.2 AA, offentlig sektor) | Automatiske kontroller og en ærlig erklæring | En manuel revision (T48), før der gøres krav på overholdelse |
| Statslig sikkerhed (NIST 800-53, ISO 27001, Cyber Essentials) | Kontrolkortlægning ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certificering og uafhængig sikring |

## Kontrakter, du får brug for, og som endnu ikke findes

1. Databehandleraftale og vilkår for international overførsel med e-mailudbyderen (T39).
2. Arbejdsbeskrivelse og fortrolighedsvilkår med firmaet, der udfører penetrationstesten (T42).
3. Vilkår for hosting og underdatabehandlere for PostgreSQL, Redis og logge.
4. Licens: se [LICENSE.md](../../../../LICENSE.md); repositoriet rummer også erklæringer om sikkerhed, styring og brug
   af AI.

## Særlige juridiske opmærksomhedspunkter

- **Sletning** anonymiserer kontakten og afvises, mens en åben deal, sag eller aktiv tilmelding til et nurture-forløb
  findes; revisionskæden gemmer optegnelser om handlinger, så afklar i din DPIA, hvordan det hænger sammen med
  anmodninger om sletning.
- **Break-glass**-læsninger af maskerede data er kun lovlige på dit dokumenterede grundlag; begrundelsesfeltet er
  bevis, ikke myndighed.
- **Scoring** af leads: hvis den nogensinde påvirker væsentlige afgørelser om personer, gælder reglerne om automatiske
  afgørelser ([regulatory](../../../../spec/regulatory.md)).
- **API-tilsagn:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) fastsætter et varsel på 180 dage,
  før en API-ændring fjernes. Det er vedligeholderens valg; hvis kunder kommer til at stole på det, så afgør, om det
  hører hjemme i en kontrakt.
- **Offentlige udsagn:** påstå ikke overholdelse, certificering eller godkendelse.
