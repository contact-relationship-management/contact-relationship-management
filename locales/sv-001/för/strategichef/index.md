# För strategichef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Den strategiska frågan

Vad är detta till för? Idag är det ett **specifikationsstyrt referens-CRM** med ett
ovanligt fullständigt säkerhets-, integritets- och granskningslager. Det är inte en produkt med
kunder, och det konkurrerar inte med mogna CRM-system
([COMPARISONS.md](../../../../COMPARISONS.md)). Valet framför dig är
om du ska behålla det som referens, eller investera i att göra det möjligt att driftsätta i
reglerade miljöer.

## Vad som verkligen skiljer sig

| Styrka | Varför det kan spela roll | Hur solid den är |
|---|---|---|
| Specifikation först: en specifikation driver både Rust-tjänsten och Svelte-gränssnittet ([spec](../../../../spec/index.md)) | Billigare att granska, portera och hålla konsekvent | Verklig; testerna hänvisar till specifikationen |
| Marknadsföring med samtycke först, tillämpad i sändningsvägen | Ett hårt krav för köpare inom hälsa och offentlig sektor | Verklig, men ingen riktig avsändare finns ännu (T39) |
| Förklarbar, regelbaserad leadpoängsättning och prognos | Köpare som måste motivera beslut om människor | Verklig; medvetet ingen maskininlärning |
| Kontakter hämtas från en identitetstjänst, dupliceras inte | Färre personuppgifter lagras, mindre yta vid intrång | Verklig; förutsätter att den tjänsten finns |
| Manipuleringssäker granskning, maskering per roll, motiverad nödåtkomst | Kontrollerna som reglerade köpare frågar om först | Implementerade och testade; inte oberoende verifierade |

## Vad som inte skiljer sig

Funktionsbredd. Offerter, avtal, telefoni, poängsättning med maskininlärning och
betalningar ligger utanför omfånget ([scope](../../../../spec/scope.md)). Den som väljer
CRM efter funktioner väljer ett annat.

## Alternativ

| Alternativ | Vad det betyder | Kostnad och risk | Belägg som behövs |
|---|---|---|---|
| A. Förbli en referens | Behåll det som undervisnings- och mönsterbibliotek | Lägst; värdet är anseende | Inget utöver dagens |
| B. Nisch i reglerad sektor | Slutför de öppna punkterna och sälj eller supportera det till hälso- och offentliga organ | Högst; kräver leverantör, juridiska villkor, externa tester, löpande säkerställande | Oberoende test (T42), tillgänglighetsgranskning (T48), riktig e-post (T39) |
| C. Komponenter, inte produkt | Erbjud granskningskedjan, maskeringen och break-glass-mönstren som återanvändbara delar | Måttlig; kräver paketering och dokumentation | Utvinningsarbete; ingen ny certifiering |

**Rekommendation: gör A nu, och finansiera det billigaste testet av B innan du förbinder dig.**
NHS-bedömningen drog slutsatsen att produkten sannolikt faller utanför DTAC:s omfång
([bedömning](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
så vägen till marknaden är inte avgjord. Lägg pengar först på att ta reda på vilken köpare
som behöver detta och vad de skulle kräva, inte på certifieringar som ingen har bett om.

## Färdplan, i den ordning beläggen föreslår

1. Oberoende säkerhetstest och manuell tillgänglighetsgranskning (T42, T48): de
   avgör om B alls är möjligt.
2. Riktig e-postleverantör (T39) när en leverantör och villkor för databehandling
   har valts; avsändargränssnittet och en SMTP-väg finns och körs mot en lokal testserver, med en signerad avregistreringslänk, en undertryckningslista och en generisk signerad webhook för utfall, men ingen leverantör är integrerad ännu ([roadmap](../../../../spec/roadmap.md)).
3. Bevisa releasesignering med en första taggad release (T49): ett minimikrav vid
   upphandling. Flödet är byggt; det har inte körts.
4. Först därefter produktfunktionerna på färdplanen (attribuering, territorier,
   SLA-klockor under kontorstid, offentligt kunskapsbasportal).

## Strategiska risker

- **Att påstå mer än som är sant.** Det snabbaste sättet att förlora en reglerad köpare är ett
  påstående om efterlevnad som beläggen inte stöder. Se [GRC-sammanfattningen](../styrning-risk-regelefterlevnad/index.md).
- **Underhåll.** Säkerhetsarbete tar aldrig slut: beroendeuppdateringar, uppdatering av digests,
  nyckelrotation, omtestning. Alternativ B binder dig till det.
- **Beroende av en identitetstjänst** som ligger utanför det här arkivet.
- **En underhållare.** Projektet har en enda underhållare och säger att granskningsdjupet
  är en persons ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Alternativ B kräver fler än en person
  som kan stå för säkerhetsarbetet.

## Beslut för dig

1. Vilket av A, B eller C, och datum för omprövning av valet.
2. Vem den första målköparen är, funnen genom samtal innan pengar läggs.
3. Om du ska finansiera oberoende säkerställande nu (se [VD-sammanfattningen](../verkställande-direktör/index.md)).
