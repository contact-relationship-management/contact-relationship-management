# Utawala, Hatari, Uzingatiaji

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Pa kuanzia

Kifurushi cha ushahidi ni [spec/compliance](../../../../spec/compliance/index.md). Kanuni
yake ya msingi ndiyo ya kuishikilia timu: udhibiti ni **Implemented** tu
wakati msimbo au usanidi unautekeleza na jaribio au ukaguzi unauthibitisha; **Deployer**
wakati shirika lako lazima litoe; **Gap** wakati hakuna lililo kweli leo.

| Hati | Itumie kwa |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR Kifungu 32, kila kimoja kimewekewa alama Implemented, Deployer au Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Tathmini ya athari ya ulinzi wa data, iliyojazwa tayari na mtiririko wa data wa mfumo huu |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Uimarishaji, nakala rudufu na urejeshaji, funguo, uandikaji kumbukumbu, mwitikio wa matukio |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Rasimu za taarifa za NHS DTAC ambazo mtengenezaji anaweza kuzipitisha |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Mapitio ya kibinafsi dhidi ya viwango vya API vya serikali ya Uingereza (rasimu, haijakaguliwa dhidi ya maandishi yaliyochapishwa) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Umbizo la makosa (RFC 9457), nini kinahesabiwa kuwa mabadiliko yanayovunja, hatua za kuacha kutumika na kipindi cha notisi |
| [Tathmini ya NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Matokeo swali kwa swali: huenda iko nje ya wigo wa DTAC, na ingeshindwa katika sehemu zinazotumika kwa kukosa ushahidi wa kishirika |
| [plan](../../../../spec/plan.md) | Hukumu ya utayari (haijawa tayari) na rejista ya mapengo |
| [tasks](../../../../spec/tasks.md) | Kazi ya kila pengo, utatuzi wake na mipaka iliyotajwa |

## Utawala

- **Haki za maamuzi:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Kanuni za uchangiaji na usaidizi wa AI ziko katika
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) na [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Udhibiti wa mabadiliko:** maelezo ya kiufundi ndiyo chanzo cha ukweli; msimbo na majaribio
  huyarejelea. CI huendeshwa kwa watoa huduma wawili, na `scripts/ci-check.sh` huirudia.
  Kanuni ndani ya hazina: usidhoofishe jaribio au mlango ili kufanya kitu kipite.
- **Ufichuzi:** [SECURITY.md](../../../../SECURITY.md) ndiyo njia ya kuripoti.

## Hatari

Mtazamo wa hatari uko katika [muhtasari wa CRO](../afisa-mkuu-wa-hatari/index.md). Kwa ufupi: hatari
kubwa zaidi zilizobaki ni usalama usiojaribiwa na watu wa nje (T42), ufikivu
usiopimwa (T48), na uainishaji wa data, ambao ni uamuzi wa mpitishaji.

## Uzingatiaji: kipi ni halisi na kipi si halisi

| Eneo | Msimamo |
|---|---|
| Udhibiti wa ufikiaji | Umetekelezwa na kujaribiwa: ufichaji kwa jukumu, ukaguzi wa ngazi ya rekodi, usomaji wa uendeshaji kwa msimamizi pekee, break-glass yenye ukaguzi wa lazima wa mtu wa pili |
| Ukaguzi | Umetekelezwa na kujaribiwa: mnyororo wa hash, kuongeza tu, ndani ya muamala, ukaguzi wa usomaji ikijumuisha orodha, orodha za mikataba na uchanganuzi. **Msambazaji:** tia nanga kichwa cha mnyororo nje ya hifadhidata |
| Haki za faragha | Zimetekelezwa: usafirishaji wa ufikiaji wa data na somo, ufutaji (hukataliwa wakati wajibu uko wazi), ufagiaji wa uhifadhi. **Msambazaji:** msingi halali, ratiba ya uhifadhi, rekodi za uchakataji |
| Usimbaji fiche | Kiasi: AES-256-GCM wa ngazi ya sehemu kwenye safu wima mbili kwa funguo za ndani, hakuna KMS. Usimbaji fiche wakati wa kuhifadhi na wakati wa usafirishaji ni wa msambazaji |
| Mnyororo wa ugavi | Umetekelezwa: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, taswira za msingi zilizobanwa kwa digest, masasisho ya Dependabot. **Pengo:** mtiririko wa toleo lililotiwa saini haujaendeshwa kwenye toleo halisi (T49) |
| Ufikivu | Kiasi: uchanganuzi wa kiotomatiki wa axe na taarifa ya uaminifu ya hadharani. **Pengo:** hakuna ukaguzi wa mikono (T48), tafsiri hazijakaguliwa |
| Uhakikisho huru | **Pengo:** hakuna (T42) |
| Sheria ya barua pepe na masoko | Njia ya utumaji inayotanguliza idhini imetekelezwa; utumaji halisi upo tu kama njia ya SMTP ya uundaji; kiungo cha kujiondoa, orodha ya kuzuia na webhook iliyotiwa saini ya matokeo vipo, lakini hakuna mtoa huduma aliyeunganishwa na makubaliano ya mchakataji na mapitio ya kisheria yako wazi (T39) |

## Udhibiti unaohitaji mmiliki katika shirika lako

1. Mapitio na tahadhari kwa ruhusa za break-glass (toa tahadhari kwa `break_glass_granted`).
2. Kutia nanga kichwa cha mnyororo wa ukaguzi na kukithibitisha kwa ratiba
   (`GET /api/audits/verify`).
3. Ushahidi wa nakala rudufu na urejeshaji, na mazoezi ya kuzungusha funguo.
4. Kushughulikia wanaojiunga, wanaohama na wanaoondoka katika mtoa utambulisho wako.
5. Uhifadhi wa jedwali la ukaguzi, ambalo hukua kulingana na trafiki ya usomaji.
6. Kulinda Redis kama hifadhidata: inashikilia data ya uchanganuzi ulio kwenye akiba.
7. Kuunganisha masasisho ya Dependabot, na kuthibitisha toleo lililotiwa saini kabla ya kulisambaza.

## Cha kukataa kuidhinisha

Kauli yoyote kwamba mfumo unazingatia, umethibitishwa, unazingatia HIPAA au umeidhinishwa
na NHS. Kauli inayoweza kutetewa ni finyu zaidi: udhibiti umepangwa, ule
uliotekelezwa umejaribiwa, na mapengo yametajwa na kufuatiliwa.
