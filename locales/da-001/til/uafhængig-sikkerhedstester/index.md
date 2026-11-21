# Uafhængig sikkerhedstester

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Hvorfor denne side findes

Hver sikkerhedskontrol i dette projekt er bygget og testet af sine forfattere (CRM-T42). Det viser, at kontrollerne
virker, som forfatterne forstod dem. Det viser ikke, at intet mangler. Hospitaler og offentlige købere kræver som regel en
test foretaget af en uafhængig, før de rummer rigtige data, og den ærlige påstand indtil da er "bygget til høj standard,
ikke testet af udenforstående"
([oversigt for den administrerende direktør](../administrerende-direktør/index.md),
[oversigt for sikkerhed](../sikkerhedsdirektør/index.md)).

Denne side er det, man giver et testfirma, og det, man kan vente at få tilbage.

## Hvad det betyder at bestå, og hvad det ikke betyder

En ren rapport siger, at et navngivet hold i et navngivet tidsrum med et navngivet omfang ikke fandt alvorlige problemer.
Den er **ikke** en certificering, ikke en akkreditering og ikke en garanti. Beskriv ikke systemet som "sikkert",
"certificeret" eller "penetrationstestet efter en standard" ud over, hvad rapporten siger
([oversigt for GRC](../governance-risiko-compliance/index.md)).

## Valg af tester

- Uafhængig af forfatterne og af enhver, der har konfigureret installationen.
- Bemandet med navngivne, erfarne testere, med relevant akkreditering for dit marked, hvor en findes (for eksempel CREST
  eller NCSC CHECK-ordningen til arbejde for den britiske regering). Spørg, hvilken akkreditering der dækker de enkelte
  personer, som udfører arbejdet, ikke kun firmaet.
- Aftaler skriftligt omfang, engagementsregler, fortrolighed, håndtering af data og ansvar, før nogen test begynder.
  Kontrakten aftales af den juridiske afdeling ([oversigt for juridisk direktør](../juridisk-direktør/index.md)).

## Hvad der skal testes

| Område | Hvad testerne bør prøve | Hvor designet er beskrevet |
|---|---|---|
| Autentificering | Forfalskede, udløbne, genafspillede tokens og tokens til den forkerte modtager; nedgradering til ikke-autentificeret; kontrollen ved start, der nægter at køre uden autentificering | [auth](../../../../spec/auth.md) |
| Autorisation | At nå poster eller handlinger, en rolle ikke bør kunne: horisontal og vertikal eskalering, kontrol på postniveau, destruktive handlinger, læsninger kun for administratorer (revision, opbevaring, break-glass-gennemgang) | [auth](../../../../spec/auth.md) |
| Eksponering af data | Maskerede værdier, der lækker gennem lister, samlede detaljelæsninger, indsigt, aggregater, fejlbeskeder, caching eller cachenøglen | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Nødadgang | At tage, forlænge eller misbruge break-glass; at gennemgå sin egen tildeling; at gennemgå to gange | [auth](../../../../spec/auth.md) (T44) |
| Revision | At ændre, slette eller omordne historik; at omgå kæden; at skjule læsninger | [audit](../../../../spec/audit.md) |
| Modstand mod misbrug | Omgåelse af hastighedsgrænse og spærring, at spærre andre brugere ude, forfalskning af headere til klientnøglen | T43 |
| Håndtering af input | Injektion (SQL, JSON, header), overstore kroppe, ugyldige markører og idempotensnøgler, genafspilning eller kollision af nøgler mellem kaldere | T51, T52 |
| Webbrugerflade | Cross-site scripting, forfalskede forespørgsler, sessionshåndtering, backend-for-frontend-proxyen, content security policy, clickjacking | UI `hooks.server.ts` |
| Infrastruktur som leveret | Containerbilleder, åbne porte, adgang til Redis og PostgreSQL, håndtering af hemmeligheder, standardværdier i konfigurationen | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Forsyningskæde | Oprindelse af afhængigheder og billeder, SBOM'erne, usignerede builds | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Forretningslogik | Omgåelse af samtykke i udsendelsesstien, sletning der efterlader data, manipulation af scoring, dobbelt oprettelse | [regulatory](../../../../spec/regulatory.md) |

**Uden for omfanget, medmindre I aftaler andet:** identitetstjenesten, der udsteder tokens (den ligger uden for dette
repositorium), hostingplatformen og denial of service i stor skala.

## Hvad testerne skal have

1. En **staging-build**, der svarer til det, der kommer til at køre i produktion: produktionskonfiguration, autentificering
   slået til, TLS, en rigtig Redis og PostgreSQL. Test ikke demoens standardværdier.
2. **Kun syntetiske data.** Seed-opgaven bygger en kundebog uden rigtige mennesker. Rigtige patient- eller borgerdata må
   aldrig være i et testmiljø.
3. **Én konto pr. rolle**, så autorisation kan testes fra hver side. Referencepolitikken er
   `config/abac-policy.reference.json`.
4. Dokumenterne, der er linket ovenfor, OpenAPI-dokumentet på `/api-docs/openapi.json` og læseadgang til repositoriet,
   så testen kan kombinere black-box og kodegennemgang.
5. De **kendte svagheder**, så testerne bruger tiden på det, der ikke allerede er skrevet ned:
   - udgivelsessignering er bygget, men har ikke kørt på en rigtig udgivelse (T49);
   - feltkryptering dækker to kolonner med lokale nøgler, ingen KMS;
   - fornyet autentificering er ikke bygget;
   - break-glass kan ikke tilbagekaldes, kun udløbe, og ingen alarm er koblet på;
   - genindlæsning af politik viser sig i cachelagret indsigt i op til 30 sekunder;
   - udlogning ved inaktivitet sker i klienten;
   - tilstand for hastighedsgrænse falder tilbage til pr. replika, hvis Redis fejler.
   Bed dem bekræfte eller afkræfte hver enkelt, ikke springe dem over.

## Engagementsregler, der skal aftales skriftligt

- Navngivne testere, datoer og kildeadresser; en kontaktperson døgnet rundt hos jer.
- Hvad der må forsøges (for eksempel ingen ødelæggelse af testens datalager uden varsel), og hvad der skal stoppe ved bevis
  for eksponering.
- Hvem der straks får besked om et kritisk fund, og hvordan, før den endelige rapport.
- Hvordan rapporten, beviserne og testernes noter opbevares og destrueres.

## Hvad I bør få tilbage

- En rapport med omfang, metode, datoer, testernes navne og akkreditering.
- Hvert fund med alvorlighed, bevis, trin til at gengive det, berørt komponent og en anbefalet løsning.
- En klar udtalelse om, hvad der **ikke** blev testet.
- En gentest af de rettede fund, medregnet i opgaven eller med pris.

## Efter rapporten

1. Sortér hvert fund i `spec/tasks.md` som en opgave med en ejer og en dato.
2. Ret kritiske og alvorlige fund før rigtige data, og lad derefter testerne bekræfte rettelserne.
3. Tilføj en regressionstest for hver rettelse. Projektets regel er ikke at svække en test eller port for at få noget til
   at bestå.
4. Optag resultatet ærligt: rapportens dato, omfanget, fundene og deres status. Opdatér planen for parathed
   ([plan](../../../../spec/plan.md)) og dokumentationspakken ([compliance](../../../../spec/compliance/index.md)).
5. Test igen efter væsentlig ændring og efter en plan, som jeres risikoejer fastsætter. Én test er et øjebliksbillede, ikke en
   tilstand.

## Rækkefølge i arbejdet

1. Luk først det, der er billigt og kendt, så testerne ikke bruger budgettet på at genopdage det: en første signeret
   udgivelse (T49), alarmering ved break-glass og ved fejl i verificering af revisionen, Redis bag TLS og legitimationsoplysninger.
2. Forbered staging-buildet, kontoene og seed-data.
3. Aftal omfang, kontrakt og engagementsregler.
4. Test; modtag kritiske fund med det samme.
5. Ret, test igen og optag.
6. Opdatér først derefter eksterne påstande, og kun til det, rapporten understøtter.

## Beslutninger til dig

1. Hvem der bestiller, og hvem der ejer fundene (se [oversigten for risikodirektøren](../risikodirektør/index.md)).
2. Hvilken akkreditering dit marked forventer af testerne.
3. Budgettet og datoen, fastsat efter, hvad en forsinkelse koster, ikke efter, hvad der er bekvemt.
4. Om der testes én gang før lancering eller efter en tilbagevendende plan.
