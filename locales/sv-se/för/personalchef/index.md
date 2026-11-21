# För personalchef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Varför människor spelar roll för den här produkten

Åtkomst till personuppgifter avgörs av roll. Programvaran tillämpar det en persons
token säger; **vem som får vilken roll, och när den tas bort, är en
personalprocess** som det här arkivet inte kan göra åt dig.

## Vad programvaran ger dina processer

- **Rollbaserad synlighet:** vissa roller ser maskerade värden för kontaktkanaler,
  affärsbelopp och penningvyer; chefer, administratörer och tjänstenivåer ser mer
  ([auth](../../../../spec/auth.md)).
- **Granskade läsningar:** att läsa personuppgifter lämnar ett spår som namnger personen.
  Personalen bör informeras om detta före driftstart.
- **Nödåtkomst:** en person kan ta en motiverad läsförhöjning på upp till 60
  minuter; en annan person måste granska den efteråt. Utbilda båda sidor.
- **Utloggning vid inaktivitet** efter 15 minuter i webbläsaren.
- **Tillgänglighet:** en offentlig redogörelse och automatiska kontroller; den manuella granskningen
  (T48) behöver fortfarande användare med funktionsnedsättning och testare av hjälpmedelsteknik.
- **Språk:** gränssnittet är översatt till 8 regioner; översättningarna har
  **inte** granskats av modersmålstalare.

## Ditt ansvar

| Område | Åtgärd |
|---|---|
| Anställning, förflyttning, avslut | Ge och ta bort roller via din identitetsleverantör; CRM-systemet vägrar utgångna token men hanterar inte konton |
| Utbildning | Dataskydd, samtyckesregler, användning och granskning av break-glass, tillgänglig praxis |
| Policyer | Godtagbar användning, information om övervakning (läsningar granskas), disciplinär väg vid missbruk av nödåtkomst |
| Granskarkapacitet | Namnge vem som granskar break-glass-beviljanden och vikarie när de är frånvarande |
| Rekrytering | Roller som programmet behöver: kontakt mot oberoende testare, klinisk säkerhetsansvarig om NHS-omfång gäller, dataskyddsansvarig, tillgänglighetsansvarig |
| Välmående | Stöd till personal som hanterar känsliga ärenden; CRM-systemet är inte ett ärendehanteringssystem men kan innehålla känsliga relationsdata |

## Öppna punkter som kräver människor

Manuell tillgänglighetsgranskning med användare av hjälpmedelsteknik (T48), granskning av översättningar
av modersmålstalare, samt ägarskap för gransknings- och larmuppgifterna ovan.
Se [tasks.md](../../../../spec/tasks.md).
