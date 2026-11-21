# Contact Relationship Management

CRM-programvara med öppen källkod för försäljning, marknadsföring och kundsupport.

Contact Relationship Management följer relationer med kunder och potentiella kunder inom försäljning, marknadsföring och support. Programmet har en post per kontakt och kräver registrerat samtycke innan ett meddelande skickas. Programvaran är inte redo för produktion: den körs på syntetiska data.

## Fördelar

- **En enda kontaktpost.** Försäljning, marknadsföring, support och analys använder samma post.
- **Registrerat samtycke.** Inget skickas till en kontakt utan registrerat samtycke. Varje e-postmeddelande innehåller en signerad avregistreringslänk.
- **Färre personuppgifter.** CRM-systemet lagrar en referens till en person i en identitetstjänst, inte en kopia av namn eller adress.
- **Förklarbara resultat.** Leadpoäng beräknas utifrån läsbara regler. Prognoser beräknas utifrån viktade pipelinesteg.
- **Internationell utformning.** Namn, adresser, kalendrar och skrivriktningar är inte bundna till ett enskilt lands format. Gränssnittet finns på åtta språk.
- **Offentlig specifikation.** Specifikationen, koden och testerna är offentliga. Dokumentationen anger vad som är byggt och vad som inte är det.

## Funktioner

- **Försäljning.** Regelbaserad leadpoängsättning, affärspipelines på en tavla och stegviktade prognoser.
- **Marknadsföring.** Segmenterade kampanjer och nurture-sekvenser. Utskicken simuleras om inte ett e-postläge har konfigurerats.
- **Support.** Ärenden med prioriteter och servicenivåfrister, samt artiklar i en kunskapsbas.
- **Analys.** Instrumentpaneler för vinstfrekvens, pipeline per steg, kampanjavkastning, servicenivåstatus och kundens livstidsvärde.
- **Datakvalitet.** Dubblettdetektering utifrån sannolikhet, med sammanslagning och ångra.
- **Styrning.** Granskningslogg, rollbaserad åtkomst och stöd för begäranden om registerutdrag och radering.

## Komma igång

1. **Hämta koden.** Klona arkivet. Node.js och pnpm krävs.
2. **Starta programmet.** Kör `pnpm install` och sedan `pnpm dev` i Svelte-katalogen.
3. **Logga in.** Använd `example@example.com`. Ingen annan tjänst krävs.
4. **Gå igenom rundturen.** Rundturen går igenom kontakter, leads, affärer, kampanjer, ärenden och ledningsvyn med fiktiva data.

[Handledningen](../../docs/tutorial-first-tour.md) beskriver varje steg. [Installera och använda](../../INSTALL.md) beskriver hela tjänsten.

## Jämförelse

| Projekt | Kategori | Skillnad |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM med öppen källkod | Fullständig produkt. Det här projektet är en mindre, specifikationsstyrd demonstration. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM med öppen källkod | Bredare funktioner. Det här projektet betonar förklarbara regler. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM med öppen källkod | Mogen och stor. Det här projektet är en kompakt referens. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Kommersiellt SaaS-CRM | Det här projektet modellerar samma arbetssätt. Ingen kod eller data delas. |

Det här projektet är en demonstration och konkurrerar inte med dessa produkter. Se [alla jämförelser](../../COMPARISONS.md).

## Rollsammanfattningar

[Rollsammanfattningarna](för/) ger en sida för varje högre roll: vad programvaran gör, vad den inte gör och vilka beslut som hör till rollen.

## Status

Programvaran är inte redo för produktion ([väg till produktion](väg-till-produktion/index.md)). Använd endast syntetiska data. Ingen riktig e-post skickas om inte ett riktigt e-postläge har konfigurerats. Inget på den här webbplatsen är juridisk rådgivning.
