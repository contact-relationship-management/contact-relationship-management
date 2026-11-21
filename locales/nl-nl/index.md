# Contact Relationship Management

Open-source CRM-software voor verkoop, marketing en klantenondersteuning.

Contact Relationship Management volgt de relaties met klanten en prospects binnen verkoop, marketing en ondersteuning. Het bewaart één record per contactpersoon en vereist vastgelegde toestemming voordat een bericht wordt verzonden. De software is niet klaar voor productie: ze draait op synthetische gegevens.

## Voordelen

- **Eén contactrecord.** Verkoop, marketing, ondersteuning en analyse gebruiken hetzelfde record.
- **Vastgelegde toestemming.** Zonder vastgelegde toestemming wordt er niets naar een contactpersoon verstuurd. Elke e-mail bevat een ondertekende afmeldlink.
- **Minder persoonsgegevens.** Het CRM bewaart een verwijzing naar een persoon in een identiteitsdienst, geen kopie van de naam of het adres.
- **Verklaarbare resultaten.** Leadscores worden berekend met leesbare regels. Prognoses worden berekend op basis van gewogen pijplijnfasen.
- **Internationaal ontwerp.** Namen, adressen, kalenders en schrijfrichtingen zijn niet gebonden aan de indeling van één land. De interface is beschikbaar in acht talen.
- **Openbare specificatie.** De specificatie, de code en de tests zijn openbaar. De documentatie vermeldt wat is gebouwd en wat niet.

## Functies

- **Verkoop.** Regelgebaseerde leadscoring, dealpijplijnen op een bord en prognoses die per fase worden gewogen.
- **Marketing.** Gesegmenteerde campagnes en nurture-reeksen. Verzenden wordt gesimuleerd, tenzij een mailmodus is ingesteld.
- **Ondersteuning.** Tickets met prioriteiten en servicelevel-deadlines, en artikelen in een kennisbank.
- **Analyse.** Dashboards voor winstpercentage, pijplijn per fase, campagnerendement, servicelevelstatus en klantlevenswaarde.
- **Gegevenskwaliteit.** Detectie van duplicaten op basis van waarschijnlijkheid, met samenvoegen en ongedaan maken.
- **Governance.** Auditlog, rolgebaseerde toegang en ondersteuning voor verzoeken om inzage en wissing.

## Aan de slag

1. **Haal de code op.** Kloon de repository. Node.js en pnpm zijn vereist.
2. **Start de applicatie.** Voer in de Svelte-map `pnpm install` uit en daarna `pnpm dev`.
3. **Meld u aan.** Gebruik `example@example.com`. Er is geen andere dienst nodig.
4. **Bekijk de rondleiding.** De rondleiding behandelt contactpersonen, leads, deals, campagnes, tickets en het managementoverzicht, met fictieve gegevens.

De [handleiding](../../docs/tutorial-first-tour.md) beschrijft elke stap. [Installeren en gebruiken](../../INSTALL.md) behandelt de volledige dienst.

## Vergelijking

| Project | Categorie | Verschil |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | Open-source CRM | Volledig product. Dit project is een kleinere, door specificaties gestuurde demonstratie. |
| [EspoCRM](../../docs/compare/espocrm.md) | Open-source CRM | Meer functies. Dit project legt de nadruk op verklaarbare regels. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | Open-source CRM | Volwassen en groot. Dit project is een compacte referentie. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Commerciële SaaS-CRM | Dit project modelleert dezelfde praktijk. Er wordt geen code of data gedeeld. |

Dit project is een demonstratie en concurreert niet met deze producten. Zie [alle vergelijkingen](../../COMPARISONS.md).

## Rolbeschrijvingen

De [rolbeschrijvingen](voor/) bieden één pagina per leidinggevende rol: wat de software doet, wat ze niet doet en welke beslissingen bij die rol horen.

## Status

Deze software is niet klaar voor productie ([pad naar productie](pad-naar-productie/index.md)). Gebruik alleen synthetische gegevens. Er wordt geen echte e-mail verzonden, tenzij een echte mailmodus is ingesteld. Niets op deze site is juridisch advies.
