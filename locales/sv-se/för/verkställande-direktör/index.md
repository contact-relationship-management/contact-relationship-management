# För verkställande direktör

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Beslutet i ett stycke

Det här är ett CRM på demonivå med ett ovanligt starkt säkerhets- och integritetsfundament. Det är **inte redo** att
hålla verkliga patient- eller medborgardata, och arkivet säger det själv ([plan.md](../../../../spec/plan.md)). Att
införa det är ett beslut att finansiera det återstående externa arbetet, inte att slå på det.

## Vad som finns

- Ett fungerande CRM för försäljning, marknadsföring, support och analys, med ett Rust-API
  och ett webbgränssnitt, byggt efter en skriven specifikation.
- Marknadsföring med samtycke först, åtkomstkontroll som döljer känsliga värden för
  roller som inte ska se dem, ett granskningsspår som visar manipulering, registerutdrag
  och radering för integritetsbegäranden, samt nödåtkomst ("break-glass") som är motiverad,
  tidsbegränsad och granskad.
- 54 av 58 spårade arbetsposter är klara ([tasks.md](../../../../spec/tasks.md)).

## Vad som står mellan detta och verklig användning

| Hinder | Varför det spelar roll | Vem kan åtgärda det |
|---|---|---|
| Ingen riktig e-postleverans (T39) | Marknadsföringen simuleras; en riktig leverantör kräver avtal och personuppgiftsbiträdesavtal | Du väljer leverantör; juridik skriver under |
| Inget oberoende säkerhetstest (T42) | Sjukhus och statliga köpare kräver oftast ett; våra egna tester är inte oberoende | Ett externt företag |
| Ingen manuell tillgänglighetsgranskning (T48) | Offentliga organ måste publicera en tillgänglighetsredogörelse de kan stå för | Personer med hjälpmedelsteknik |
| Signering av releaser är byggd men har inte körts på en riktig release, och ingen granskare har kontrollerat utkastet till API-standarder (T49) | Upphandlingschecklistor frågar efter båda | Utveckling (tagga en första release); en extern granskare |

## Beslut som är dina

1. **Vilken marknad först.** Bedömningen mot NHS Digital Technology Assessment Criteria
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   drar slutsatsen att produkten sannolikt faller utanför DTAC:s omfång och skulle underkännas
   i de avsnitt som gäller på grund av bristande organisatoriska belägg (utsedd
   klinisk säkerhetsansvarig, certifieringar). Välj väg innan du lägger pengar på den.
2. **Om du ska finansiera oberoende säkerställande** (T42, T48). Utan det är det ärliga
   påståendet "byggd efter hög standard, otestad av utomstående".
3. **Vem som äger risken** när verkliga data hålls. Se sammanfattningarna för [riskchef](../riskchef/index.md)
   och [juridikchef](../juridikchef/index.md).

## Vad du inte ska säga externt

Påstå inte regelefterlevnad, certifiering, "HIPAA-kompatibel" eller "NHS-godkänd".
Bevispaketet ([compliance](../../../../spec/compliance/index.md)) stöder ett snävare, sant
påstående: kontroller är kartlagda, de som är implementerade är testade, och luckor är namngivna.
