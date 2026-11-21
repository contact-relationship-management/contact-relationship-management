# Oberoende säkerhetstestare

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Varför detta finns

Varje säkerhetskontroll i det här projektet byggdes och testades av sina upphovspersoner
(CRM-T42). Det visar att kontrollerna fungerar så som upphovspersonerna förstod dem. Det visar
inte att inget saknas. Sjukhus och statliga köpare kräver oftast
ett test av någon oberoende innan de håller verkliga data, och det ärliga påståendet
tills dess är "byggd efter hög standard, otestad av utomstående"
([VD-sammanfattningen](../verkställande-direktör/index.md),
[säkerhetssammanfattningen](../säkerhetschef/index.md)).

Den här sidan är vad som ska lämnas till ett testföretag, och vad som ska förväntas tillbaka.

## Vad att klara testet betyder, och vad det inte betyder

En ren rapport säger att ett namngivet team, under en namngiven tid, med ett namngivet omfång, inte hittade några
allvarliga problem. Det är **inte** en certifiering, inte en ackreditering och inte en
garanti. Beskriv inte systemet som "säkert", "certifierat" eller "penetrationstestat
enligt en standard" utöver vad rapporten anger
([GRC-sammanfattningen](../styrning-risk-regelefterlevnad/index.md)).

## Att välja testare

- Oberoende av upphovspersonerna och av alla som konfigurerade driftsättningen.
- Bemannat med namngivna, erfarna testare, med relevant ackreditering för din
  marknad där en sådan finns (till exempel CREST, eller NCSC CHECK-systemet för
  brittiskt statligt arbete). Fråga vilken ackreditering som täcker de individer som utför
  arbetet, inte bara företaget.
- Kommer skriftligt överens om omfång, engagemangsregler, sekretess, datahantering
  och ansvar innan något test påbörjas. Avtalet är juridikteamets att
  komma överens om ([juridikchefens sammanfattning](../juridikchef/index.md)).

## Vad som ska testas

| Område | Vad testarna bör försöka | Var designen beskrivs |
|---|---|---|
| Autentisering | Förfalskade, utgångna, återuppspelade och token med fel målgrupp; nedgradering till oautentiserat; startkontrollen som vägrar köra med autentisering avstängd | [auth](../../../../spec/auth.md) |
| Auktorisering | Att nå poster eller åtgärder som en roll inte ska: horisontell och vertikal behörighetseskalering, kontroller på postnivå, destruktiva åtgärder, läsningar endast för admin (granskning, gallring, break-glass-granskning) | [auth](../../../../spec/auth.md) |
| Dataexponering | Maskerade värden som läcker via listor, sammansatta detaljläsningar, insikter, aggregat, felmeddelanden, cachning eller cachenyckeln | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Nödåtkomst | Att ta, förlänga eller missbruka break-glass; att granska sitt eget beviljande; att granska två gånger | [auth](../../../../spec/auth.md) (T44) |
| Granskning | Att ändra, radera eller ordna om historik; att kringgå kedjan; att dölja läsningar | [audit](../../../../spec/audit.md) |
| Motståndskraft mot missbruk | Kringgående av hastighetsgräns och spärr, utlåsning av andra användare, header-förfalskning för klientnyckeln | T43 |
| Hantering av indata | Injektion (SQL, JSON, header), överstora kroppar, felaktiga markörer och idempotensnycklar, återuppspelning eller kollision av nycklar mellan anropare | T51, T52 |
| Webbgränssnitt | Cross-site scripting, förfalskning av begäranden, sessionshantering, backend-for-frontend-proxyn, content security policy, clickjacking | Gränssnittets `hooks.server.ts` |
| Infrastruktur som levererad | Containeravbilder, exponerade portar, nåbarhet för Redis och PostgreSQL, hantering av hemligheter, konfigurationens standardvärden | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Leveranskedja | Ursprung för beroenden och avbilder, SBOM:erna, osignerade byggen | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Affärslogik | Kringgående av samtycke i sändningsvägen, radering som lämnar kvar data, manipulation av poängsättning, skapande av dubbletter | [regulatory](../../../../spec/regulatory.md) |

**Utanför omfånget om ni inte kommer överens om annat:** identitetstjänsten som utfärdar
token (den ligger utanför det här arkivet), hostingplattformen och överbelastningsattacker
i stor volym.

## Vad testarna ska få

1. Ett **staging-bygge** som motsvarar det som ska köras i produktion: produktionskonfiguration,
   autentisering på, TLS, en riktig Redis och PostgreSQL. Testa inte
   demons standardvärden.
2. **Endast syntetiska data.** Seed-uppgiften bygger ett kundregister utan verkliga
   personer. Verkliga patient- eller medborgardata får aldrig finnas i en testmiljö.
3. **Ett konto per roll**, så att auktorisering kan testas från varje sida. Referenspolicyn
   är `config/abac-policy.reference.json`.
4. Dokumenten som länkas ovan, OpenAPI-dokumentet på `/api-docs/openapi.json`
   och läsåtkomst till arkivet, så att testet kan kombinera black-box och kod-
   granskning.
5. De **kända svagheterna**, så att testarna lägger sin tid på det som inte redan är
   nedskrivet:
   - releasesignering är byggd men har inte körts på en riktig release (T49);
   - fältkryptering täcker två kolumner med lokala nycklar, ingen KMS;
   - förnyad autentisering (step-up) är inte byggd;
   - break-glass kan inte återkallas, bara löpa ut, och inget larm är kopplat;
   - policyomladdningar syns i cachade insikter i upp till 30 sekunder;
   - utloggning vid inaktivitet sker på klientsidan;
   - hastighetsgränsens tillstånd faller tillbaka på per replika om Redis fallerar.
   Be dem bekräfta eller vederlägga var och en, inte hoppa över dem.

## Engagemangsregler att komma överens om skriftligt

- Namngivna testare, datum och källadresser; en kontakt dygnet runt på din sida.
- Vad som får prövas (till exempel ingen förstörelse av testdatalagret
  utan förvarning) och vad som måste stanna vid bevis på exponering.
- Vem som genast underrättas om ett kritiskt fynd, och hur, före slutrapporten.
- Hur rapporten, beläggen och eventuella anteckningar från testarna lagras och förstörs.

## Vad du bör få tillbaka

- En rapport med omfång, metod, datum, testarnas namn och ackreditering.
- Varje fynd med allvarlighetsgrad, belägg, steg för att återskapa, berörd komponent
  och en rekommenderad åtgärd.
- Ett tydligt uttalande om vad som **inte** testades.
- Ett omtest av de åtgärdade fynden, inkluderat i uppdraget eller prissatt.

## Efter rapporten

1. Triagera varje fynd i `spec/tasks.md` som en uppgift med ägare och datum.
2. Åtgärda kritiska och höga fynd före alla verkliga data, och låt sedan testarna
   bekräfta åtgärderna.
3. Lägg till ett regressionstest för varje åtgärd. Projektets regel är att inte försvaga ett test
   eller en grind för att få något att gå igenom.
4. Anteckna resultatet ärligt: rapportens datum, omfånget, fynden och
   deras status. Uppdatera beredskapsplanen ([plan](../../../../spec/plan.md)) och
   efterlevnadspaketet ([compliance](../../../../spec/compliance/index.md)).
5. Testa om efter väsentlig förändring, och enligt ett schema som din riskägare fastställer.
   Ett test är en ögonblicksbild, inte ett tillstånd.

## Arbetsordning

1. Stäng först det som är billigt och känt, så att testarna inte spenderar budgeten
   på att återupptäcka det: en första signerad release (T49), larm på break-glass
   och på misslyckad granskningsverifiering, Redis bakom TLS och autentiseringsuppgifter.
2. Förbered staging-bygget, kontona och seed-data.
3. Kom överens om omfång, avtal och engagemangsregler.
4. Testa; ta emot kritiska fynd direkt.
5. Åtgärda, testa om och anteckna.
6. Först därefter uppdatera externa påståenden, och endast till det rapporten stöder.

## Beslut för dig

1. Vem som beställer och vem som äger fynden (se
   [riskchefens sammanfattning](../riskchef/index.md)).
2. Vilken ackreditering din marknad förväntar sig av testarna.
3. Budget och datum, bestämda av vad en försening kostar snarare än av vad som är
   bekvämt.
4. Om man ska testa en gång före lansering eller enligt ett återkommande schema.
