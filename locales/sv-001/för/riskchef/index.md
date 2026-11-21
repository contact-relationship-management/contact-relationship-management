# För riskchef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Riskläge

**Sammantaget: inte redo för reglerade data.** Programvaran är väl kontrollerad för
sitt stadium, men de huvudsakliga kvarvarande riskerna är obegränsade av tester och processer
som endast den införande organisationen eller externa parter kan tillhandahålla.

## Främsta risker

| # | Risk | Underlag för bedömningen | Begränsning i arkivet | Kvarstående |
|---|---|---|---|---|
| 1 | Oupptäckta sårbarheter | Inget oberoende penetrationstest (T42) | Egna tester, grindar för leveranskedjan, strikta headers/CSP | **Hög** tills testad |
| 2 | Känsliga fält exponeras för fel roll | Komplexa åtkomstregler | Rollbaserad maskering av poster, aggregat och insikter; ett persona-matristest; ett läckage hittades och åtgärdades i kontodetaljer (T47) | Medel: maskeringen är testad, inte oberoende verifierad |
| 3 | Internt missbruk av nödåtkomst | Break-glass höjer läsrättigheter | Skriftligt skäl, tak på 60 minuter, skrivskyddad, obligatorisk granskning av en andra person, granskad | Medel: inget larm är kopplat; ingen återkallning |
| 4 | Granskningsspåret ändras | Kräver databasåtkomst | Hashkedja plus utlösare som bara läggs till; rapporterar det första brottet | Låg–medel: huvudets hash måste förankras utanför databasen |
| 5 | Olaglig marknadsföring | Samtyckesfel | Sändningsvägen tillämpar samtycke, inte gränssnittet; oföränderlig samtyckeshistorik | Medel tills en riktig leverantör är integrerad och granskad (T39) |
| 6 | Tillgänglighet / missbruk | Offentligt API | Hastighetsgräns och spärr, delade via Redis | Medel: gränserna faller tillbaka per replika om Redis fallerar |
| 7 | Bristande tillgänglighetsefterlevnad | Skyldighet i offentlig sektor | Automatiska axe-skanningar; ärlig tillgänglighetsredogörelse | **Okvantifierad** tills manuell granskning (T48) |
| 8 | Leveranskedja | Beroenden och avbilder | `cargo deny/audit`, `pnpm audit`, SBOM, digestlåsningar som Dependabot uppdaterar, ett nyckellöst signerat releaseflöde | Medel: signeringen har inte körts på en riktig release (T49) |
| 9 | Dubbletter från omförsök | Idempotensnycklar är frivilliga i API:et (T52) | Återuppspelning för klienter som skickar nyckeln; det medföljande gränssnittet gör det alltid | Låg |
| 10 | Felklassificering av data | Driftsättaren avgör vad en "kontakt" är | Avgränsningsnotering och DPIA-mall | Beror på dig |

## Luckor som förblir öppna till sin natur

T39 (riktig e-postleverantör), T42 (oberoende säkerställande), T48 (manuell
tillgänglighetsgranskning), T49 (en första signerad release, en granskares kontroll av API-utkastet). Se [tasks.md](../../../../spec/tasks.md)
och [plan.md](../../../../spec/plan.md).

## Föreslagna riskåtgärder

Registrera risk 1, 7 och 10 formellt; kräv T42 före alla verkliga data; utse en
ägare för granskning och larm av break-glass; fastställ hur ofta granskningshuvudet ska förankras.
