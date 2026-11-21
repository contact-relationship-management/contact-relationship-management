# Kwa Afisa Mkuu wa Hatari

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Msimamo wa hatari

**Kwa ujumla: haijawa tayari kwa data iliyodhibitiwa.** Programu imedhibitiwa vyema kwa
hatua yake, lakini hatari kuu zilizobaki hazijapunguzwa na majaribio na mchakato
ambao shirika linalopitisha au watu wa nje pekee wanaweza kutoa.

## Hatari kuu

| # | Hatari | Msingi wa ukadiriaji | Kupunguza ndani ya hazina | Iliyobaki |
|---|---|---|---|---|
| 1 | Udhaifu usiogunduliwa | Hakuna jaribio huru la kuvamia (T42) | Majaribio yetu wenyewe, milango ya mnyororo wa ugavi, vichwa/CSP vikali | **Juu** hadi ijaribiwe |
| 2 | Kufichuliwa kwa sehemu nyeti kwa jukumu lisilo sahihi | Kanuni changamano za ufikiaji | Ufichaji kwa jukumu kwenye rekodi, jumla na uchanganuzi; jaribio la matriki ya wahusika; uvujaji uliopatikana na kurekebishwa kwenye maelezo ya akaunti (T47) | Wastani: ufichaji umejaribiwa, haujathibitishwa kwa uhuru |
| 3 | Matumizi mabaya ya ndani ya ufikiaji wa dharura | Break-glass hupandisha usomaji | Sababu iliyoandikwa, kikomo cha dakika 60, usomaji tu, ukaguzi wa lazima wa mtu wa pili, hukaguliwa | Wastani: hakuna tahadhari iliyounganishwa; hakuna kufuta |
| 4 | Njia ya ukaguzi kubadilishwa | Inahitaji ufikiaji wa hifadhidata | Mnyororo wa hash pamoja na kichocheo cha kuongeza tu; huripoti uvunjikaji wa kwanza | Chini-Wastani: hash ya kichwa lazima itiwe nanga nje ya hifadhidata |
| 5 | Masoko yasiyo halali | Makosa ya idhini | Njia ya utumaji hutekeleza idhini, si UI; historia ya idhini isiyobadilika | Wastani hadi mtoa huduma halisi aunganishwe na akaguliwe (T39) |
| 6 | Upatikanaji / matumizi mabaya | API ya umma | Kikomo cha kasi na kufungiwa, vinavyoshirikiwa kupitia Redis | Wastani: vikomo hurudi kwa kila nakala Redis ikishindwa |
| 7 | Kutozingatia ufikivu | Wajibu wa sekta ya umma | Uchanganuzi wa kiotomatiki wa axe; taarifa ya uaminifu ya ufikivu | **Haijapimwa** hadi ukaguzi wa mikono (T48) |
| 8 | Mnyororo wa ugavi | Tegemezi na taswira | `cargo deny/audit`, `pnpm audit`, SBOM, digest zilizobanwa zinazosasishwa na Dependabot, mtiririko wa toleo uliotiwa saini bila ufunguo | Wastani: utiaji saini haujaendeshwa kwenye toleo halisi (T49) |
| 9 | Rekodi nakala mbili kutokana na kujaribu tena | Funguo za idempotency ni hiari kwenye API (T52) | Kurudia kwa wateja wanaotuma ufunguo; UI iliyojumuishwa daima hutuma | Chini |
| 10 | Uainishaji mbaya wa data | Msambazaji huamua "mwasiliani" ni nini | Maelezo ya wigo na kiolezo cha DPIA | Inategemea wewe |

## Mapengo yanayobaki wazi kwa asili

T39 (mtoa huduma halisi wa barua pepe), T42 (uhakikisho huru), T48 (ukaguzi wa mikono wa
ufikivu), T49 (toleo la kwanza lililotiwa saini, ukaguzi wa mkaguzi wa rasimu ya API). Tazama [tasks.md](../../../../spec/tasks.md)
na [plan.md](../../../../spec/plan.md).

## Hatua za hatari zinazopendekezwa

Sajili hatari 1, 7 na 10 rasmi; hitaji T42 kabla ya data yoyote halisi; weka
mmiliki wa ukaguzi na tahadhari za break-glass; weka mzunguko wa kutia nanga kichwa cha ukaguzi.
