# Contact Relationship Management

CRM-software med åben kildekode til salg, marketing og kundesupport.

Contact Relationship Management følger kunde- og emnerelationer på tværs af salg, marketing og support. Systemet har én post for hver kontakt og kræver registreret samtykke, før en besked sendes. Softwaren er ikke klar til produktion: den kører på syntetiske data.

## Fordele

- **Én kontaktpost.** Salg, marketing, support og analyse bruger den samme post.
- **Registreret samtykke.** Intet sendes til en kontakt uden registreret samtykke. Hver e-mail indeholder et signeret afmeldingslink.
- **Færre personoplysninger.** CRM-systemet gemmer en henvisning til en person i en identitetstjeneste, ikke en kopi af navn eller adresse.
- **Forklarlige resultater.** Lead-scores beregnes ud fra læsbare regler. Prognoser beregnes ud fra vægtede faser i pipelinen.
- **International opbygning.** Navne, adresser, kalendere og skriveretninger er ikke bundet til ét lands format. Brugerfladen findes på otte sprog.
- **Offentlig specifikation.** Specifikationen, koden og testene er offentlige. Dokumentationen angiver, hvad der er bygget, og hvad der ikke er.

## Funktioner

- **Salg.** Regelbaseret lead-scoring, deal-pipelines på en tavle og prognoser vægtet efter fase.
- **Marketing.** Segmenterede kampagner og nurture-forløb. Udsendelsen er simuleret, medmindre en mailtilstand er konfigureret.
- **Support.** Sager med prioriteter og frister for serviceniveau samt artikler i vidensbasen.
- **Analyse.** Dashboards for vinderrate, pipeline pr. fase, afkast af kampagner, status for serviceniveau og kundens livstidsværdi.
- **Datakvalitet.** Registrering af dubletter efter sandsynlighed, med sammenlægning og fortryd.
- **Styring.** Revisionslog, rollebaseret adgang og understøttelse af anmodninger om indsigt og sletning.

## Kom i gang

1. **Hent koden.** Klon repositoriet. Node.js og pnpm er påkrævet.
2. **Start programmet.** Kør `pnpm install` og derefter `pnpm dev` i Svelte-mappen.
3. **Log ind.** Brug `example@example.com`. Ingen anden tjeneste er nødvendig.
4. **Gennemgå rundvisningen.** Rundvisningen dækker kontakter, leads, deals, kampagner, sager og ledelsesoversigten med fiktive data.

[Vejledningen](../../docs/tutorial-first-tour.md) beskriver hvert trin. [Installation og brug](../../INSTALL.md) dækker hele tjenesten.

## Sammenligning

| Projekt | Kategori | Forskel |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM med åben kildekode | Fuldt produkt. Dette projekt er en mindre, specifikationsdrevet demonstration. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM med åben kildekode | Flere funktioner. Dette projekt lægger vægt på forklarlige regler. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM med åben kildekode | Modent og stort. Dette projekt er en kompakt reference. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Kommercielt SaaS-CRM | Dette projekt modellerer den samme praksis. Hverken kode eller data deles. |

Dette projekt er en demonstration og konkurrerer ikke med disse produkter. Se [alle sammenligninger](../../COMPARISONS.md).

## Rolleoversigter

[Rolleoversigterne](til/) giver én side for hver ledende rolle: hvad softwaren gør, hvad den ikke gør, og hvilke beslutninger der tilhører den rolle.

## Status

Denne software er ikke klar til produktion ([vej til produktion](vej-til-produktion/index.md)). Brug kun syntetiske data. Der sendes ingen rigtig e-mail, medmindre en rigtig mailtilstand er konfigureret. Intet på dette websted er juridisk rådgivning.
