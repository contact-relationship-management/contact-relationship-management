# Mjaribu Huru wa Usalama

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Kwa nini hii ipo

Kila udhibiti wa usalama katika mradi huu ulijengwa na kujaribiwa na waandishi wake
(CRM-T42). Hilo linaonyesha udhibiti unafanya kazi kama waandishi walivyoelewa. Haliashirii
kwamba hakuna kinachokosekana. Hospitali na wanunuzi wa serikali kwa kawaida huhitaji
jaribio la mtu huru kabla hawajashikilia data halisi, na dai la uaminifu
hadi wakati huo ni "imejengwa kwa kiwango cha juu, haijajaribiwa na watu wa nje"
([muhtasari wa CEO](../afisa-mkuu-mtendaji/index.md),
[muhtasari wa usalama](../afisa-mkuu-wa-usalama/index.md)).

Ukurasa huu ni kile cha kumkabidhi kampuni ya majaribio, na cha kutarajia kurudishiwa.

## Kupita kunamaanisha nini, na kisichomaanisha

Ripoti safi inasema timu iliyotajwa, ndani ya muda uliotajwa, kwa wigo uliotajwa, haikupata
masuala makubwa. **Si** uthibitisho, si idhini rasmi na si
dhamana. Usielezee mfumo kama "salama", "uliothibitishwa" au "uliojaribiwa kuvamiwa
kwa kiwango" zaidi ya ripoti inavyosema
([muhtasari wa GRC](../utawala-hatari-uzingatiaji/index.md)).

## Kuchagua mjaribu

- Huru kutoka kwa waandishi, na kutoka kwa yeyote aliyesanidi usambazaji.
- Wenye wafanyakazi wajaribu waliotajwa na wenye uzoefu, wenye idhini rasmi inayofaa kwa
  soko lako pale inapotumika (kwa mfano CREST, au mpango wa NCSC CHECK kwa
  kazi za serikali ya Uingereza). Uliza idhini rasmi gani inawafunika watu binafsi wanaofanya
  kazi, si kampuni tu.
- Wanakubali kwa maandishi wigo, kanuni za ushiriki, usiri, ushughulikiaji wa data,
  na dhima kabla ya jaribio lolote kuanza. Mkataba ni wa timu ya sheria kukubaliana
  ([muhtasari wa CLO](../afisa-mkuu-wa-sheria/index.md)).

## Cha kujaribu

| Eneo | Wajaribu wanapaswa kujaribu nini | Mahali muundo unaelezwa |
|---|---|---|
| Uthibitishaji | Tokeni zilizoghushiwa, zilizoisha muda, zilizochezwa tena na za hadhira isiyo sahihi; kushushwa hadi kutothibitishwa; ukaguzi wa kuanza unaokataa kuendesha uthibitishaji ukiwa umezimwa | [auth](../../../../spec/auth.md) |
| Idhini | Kufikia rekodi au vitendo ambavyo jukumu halipaswi kufikia: kupanda kwa mlalo na kwa wima, ukaguzi wa ngazi ya rekodi, vitendo vya kuharibu, usomaji wa msimamizi pekee (ukaguzi, uhifadhi, mapitio ya break-glass) | [auth](../../../../spec/auth.md) |
| Kufichuliwa kwa data | Thamani zilizofichwa zikivuja kupitia orodha, usomaji wa maelezo uliounganishwa, uchanganuzi, jumla, jumbe za makosa, akiba, au ufunguo wa akiba | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Ufikiaji wa dharura | Kuchukua, kuongeza au kutumia vibaya break-glass; kukagua ruhusa yako mwenyewe; kukagua mara mbili | [auth](../../../../spec/auth.md) (T44) |
| Ukaguzi | Kubadilisha, kufuta au kupanga upya historia; kupita mnyororo; kuficha usomaji | [audit](../../../../spec/audit.md) |
| Upinzani dhidi ya matumizi mabaya | Kukwepa kikomo cha kasi na kufungiwa, kuwafungia watumiaji wengine, kughushi vichwa kwa ufunguo wa mteja | T43 |
| Ushughulikiaji wa ingizo | Uingizaji (SQL, JSON, kichwa), miili mikubwa kupita kiasi, vielekezi na funguo za idempotency zilizoharibika, kurudia au kugongana kwa funguo kati ya wapigaji | T51, T52 |
| Sehemu ya mbele ya wavuti | Cross-site scripting, ughushi wa ombi, ushughulikiaji wa kipindi, proksi ya backend-for-frontend, sera ya usalama wa maudhui, clickjacking | UI `hooks.server.ts` |
| Miundombinu kama inavyosafirishwa | Taswira za kontena, milango iliyofichuliwa, kufikika kwa Redis na PostgreSQL, ushughulikiaji wa siri, thamani chaguo-msingi za usanidi | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Mnyororo wa ugavi | Asili ya tegemezi na taswira, SBOM, ujenzi usiotiwa saini | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Mantiki ya biashara | Kukwepa idhini katika njia ya utumaji, ufutaji unaoacha data, kuchezea utoaji alama, uundaji wa nakala mbili | [regulatory](../../../../spec/regulatory.md) |

**Nje ya wigo isipokuwa mkubaliane vinginevyo:** huduma ya utambulisho inayotoa
tokeni (iko nje ya hazina hii), jukwaa la upangishaji, na kunyimwa huduma
kwa kiasi kikubwa.

## Cha kuwapa wajaribu

1. **Ujenzi wa staging** unaolingana na kile kitakachoendeshwa katika uzalishaji: usanidi wa
   uzalishaji, uthibitishaji umewashwa, TLS, Redis na PostgreSQL halisi. Usijaribu
   thamani chaguo-msingi za onyesho.
2. **Data bandia tu.** Kazi ya mbegu hujenga kitabu cha biashara kisicho na watu
   halisi. Data halisi ya wagonjwa au wananchi haipaswi kamwe kuwa katika mazingira ya majaribio.
3. **Akaunti moja kwa kila jukumu**, ili idhini ijaribiwe kutoka kila upande. Sera
   ya rejeleo ni `config/abac-policy.reference.json`.
4. Hati zilizounganishwa hapo juu, hati ya OpenAPI kwenye `/api-docs/openapi.json`,
   na ufikiaji wa kusoma wa hazina, ili jaribio liweze kuchanganya black-box na ukaguzi wa
   msimbo.
5. **Udhaifu unaojulikana**, ili wajaribu watumie muda wao kwa kile ambacho
   hakijaandikwa tayari:
   - utiaji saini wa toleo umejengwa lakini haujaendeshwa kwenye toleo halisi (T49);
   - usimbaji fiche wa sehemu unashughulikia safu wima mbili kwa funguo za ndani, hakuna KMS;
   - uthibitishaji upya wa hatua ya juu haujajengwa;
   - break-glass haiwezi kufutwa, inaisha muda tu, na hakuna tahadhari iliyounganishwa;
   - upakiaji upya wa sera huonekana katika uchanganuzi ulio kwenye akiba hadi sekunde 30;
   - kutoka kiotomatiki kwa kutofanya kazi hufanyika upande wa mteja;
   - hali ya kikomo cha kasi hurudi kwa kila nakala Redis ikishindwa.
   Waombe wathibitishe au wakanushe kila kimoja, wasivirukie.

## Kanuni za ushiriki za kukubaliwa kwa maandishi

- Wajaribu waliotajwa, tarehe na anwani za chanzo; mwasiliani wa saa 24 upande wako.
- Kinachoweza kujaribiwa (kwa mfano, kutoharibu hifadhi ya data ya majaribio
  bila notisi) na kinachopaswa kusimama kwenye uthibitisho wa kufichuliwa.
- Nani anaambiwa mara moja kuhusu matokeo muhimu sana, na jinsi gani, kabla ya ripoti ya mwisho.
- Jinsi ripoti, ushahidi na maelezo yoyote ya wajaribu yanavyohifadhiwa na kuharibiwa.

## Cha kupata kurudi

- Ripoti yenye wigo, njia, tarehe, majina ya wajaribu na idhini rasmi.
- Kila matokeo likiwa na ukali, ushahidi, hatua za kuzalisha tena, kipengele
  kilichoathiriwa na suluhisho linalopendekezwa.
- Taarifa wazi ya kile **kisichojaribiwa**.
- Jaribio upya la matokeo yaliyorekebishwa, likijumuishwa katika ushiriki au likiwa na bei.

## Baada ya ripoti

1. Pangilia kila matokeo katika `spec/tasks.md` kama kazi yenye mmiliki na tarehe.
2. Rekebisha matokeo muhimu sana na ya juu kabla ya data yoyote halisi, kisha wajaribu
   wathibitishe marekebisho.
3. Ongeza jaribio la regression kwa kila marekebisho. Kanuni ya mradi ni kutodhoofisha jaribio
   au mlango ili kufanya kitu kipite.
4. Rekodi matokeo kwa uaminifu: tarehe ya ripoti, wigo, matokeo na
   hali yake. Sasisha mpango wa utayari ([plan](../../../../spec/plan.md)) na
   kifurushi cha uzingatiaji ([compliance](../../../../spec/compliance/index.md)).
5. Jaribu upya baada ya mabadiliko makubwa, na kwa ratiba inayowekwa na mmiliki wako wa hatari.
   Jaribio moja ni picha ya wakati, si hali ya kudumu.

## Mpangilio wa kazi

1. Maliza kwanza kilicho rahisi na kinachojulikana, ili wajaribu wasitumie bajeti
   kukigundua upya: toleo la kwanza lililotiwa saini (T49), tahadhari kwa break-glass
   na kwa kushindwa kwa uthibitishaji wa ukaguzi, Redis nyuma ya TLS na vitambulisho.
2. Andaa ujenzi wa staging, akaunti na data ya mbegu.
3. Kubaliana wigo, mkataba na kanuni za ushiriki.
4. Jaribu; pokea matokeo muhimu sana mara moja.
5. Rekebisha, jaribu upya, na rekodi.
6. Kisha tu sasisha madai ya nje, na kwa kile ambacho ripoti inaunga mkono tu.

## Maamuzi kwako

1. Nani anaagiza na nani anamiliki matokeo (tazama
   [muhtasari wa CRO](../afisa-mkuu-wa-hatari/index.md)).
2. Idhini rasmi gani soko lako linatarajia kwa wajaribu.
3. Bajeti na tarehe, vilivyowekwa na gharama ya kuchelewa badala ya kile ambacho
   kinafaa.
4. Kama kujaribu mara moja kabla ya uzinduzi au kwa ratiba ya kujirudia.
