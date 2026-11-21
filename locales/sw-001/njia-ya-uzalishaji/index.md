# Njia ya uzalishaji

> Contact Relationship Management bado haijawa tayari kwa uzalishaji. Ukurasa huu unaeleza uzalishaji unamaanisha nini hapa,
> na kila sehemu imefikia wapi.

Programu inafanya kazi kwa data bandia. Barua pepe huigizwa isipokuwa hali halisi ya barua pepe imewekwa. Usiweke data binafsi
halisi ndani yake.

## Uzalishaji unamaanisha nini

Usambazaji wa uzalishaji una data ya watu halisi na hutuma ujumbe halisi kwao. Kwa mradi huu hilo linahitaji yote yafuatayo:

- **Kuingia halisi.** Kila ombi linathibitishwa, na kanuni za ufikiaji zinalingana na majukumu ya shirika linalosambaza.
- **Utumaji halali.** Kila ujumbe una idhini iliyorekodiwa na msingi halali chini ya sheria ya kila mahali unakotumwa, na
  hupitia mtoa huduma halisi wa barua pepe anayeshughulikia barua zilizorudi, malalamiko na kujiondoa.
- **Uhakikisho huru.** Watu walio nje ya mradi wamejaribu usalama na ufikivu, na matokeo yamerekebishwa au kukubaliwa.
- **Uendeshaji.** Nakala rudufu zilizorejeshwa kwa mafanikio, ufuatiliaji, mchakato wa matukio, na mtu aliye zamu.
- **Ushahidi wa uthabiti.** Uendeshaji mrefu chini ya mzigo halisi, kwenye maunzi yanayofanana na ya usambazaji, bila makosa
  wala kupungua kwa kasi.

Utayari wa uzalishaji huamuliwa kwa kila usambazaji. Msambazaji anaweza kufunga baadhi ya milango kwa ajili ya shirika lake na
wapokeaji wake tu. Tathmini ya kihandisi inayotokana na ukurasa huu ni [spec/plan.md](../../../spec/plan.md). Milango iliyo wazi
iko katika [spec/tasks.md](../../../spec/tasks.md).

## Utayari

Kila asilimia ni makadirio yaliyofanywa tarehe 2026-11-13 kutoka orodha ya kazi, si kipimo.

- **Uthibitishaji na udhibiti wa ufikiaji** (70%): Kuingia kwa tokeni za PASETO au Keycloak/OIDC, kanuni za ufikiaji zinazotegemea
  sifa zenye ukaguzi wa ngazi ya rekodi, na hali ya uzalishaji inayokataa kuanza bila uthibitishaji zimejengwa. Hakuna
  usambazaji halisi ambao umeviwasha kwa sifa zake wenyewe, na jedwali la utekelezaji bado halishughulikii kila kidhibiti
  (CRM-G1).
- **Utumaji wa barua pepe** (40%): Mtumaji wa kuigiza ndiye chaguo-msingi, na mtumaji wa SMTP yupo. Hakuna kiunganishi cha mtoa
  huduma wa barua pepe chenye webhook za barua zilizorudi na malalamiko wala orodha ya kuzuia (CRM-T39).
- **Ulinzi wa data na sheria ya masoko ya kielektroniki** (15%): Rekodi za idhini, kufikia data na somo, kufuta na kuhifadhi
  zimejengwa. Hakuna sheria ya eneo lolote iliyosomwa kwa mradi huu, na mapitio ya njia halisi ya utumaji yako wazi (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Uhakikisho huru wa usalama** (10%): Wigo wa jaribio la kuvamia umeandikwa. Hakuna jaribio la kuvamia, uchambuzi rasmi wa
  hatari wala idhini rasmi iliyofanyika (CRM-T42). Ukaguzi wa usalama wa AI si uhakikisho huru.
- **Ufikivu** (50%): Ukaguzi wa kiotomatiki wa axe-core dhidi ya WCAG 2.2 A na AA unapita kwenye kila njia ya ukurasa. Hakuna
  ukaguzi wa mikono kwa kutumia kibodi pekee, visomaji vya skrini au ukuzaji wa 400% uliofanyika (CRM-T48).
- **Tafsiri** (30%): Kiolesura kinapatikana kwa lugha nane. Maandishi ya lugha zisizo Kiingereza yameandikwa na mashine, na
  hakuna mzungumzaji aliyeyakagua.
- **Uendeshaji** (30%): Mwongozo wa uendeshaji unashughulikia nakala rudufu na urejeshaji, usimamizi wa funguo, usimbaji fiche
  wakati wa kuhifadhi, mwitikio wa matukio na uhifadhi wa kumbukumbu. Hakuna urejeshaji uliojaribiwa, na orodha ya ukaguzi ya
  kuanza kazi ya mwongozo iko wazi.
- **Utendaji na uthabiti** (20%): Majaribio ya mzigo na jaribio la uthabiti la muda mrefu yapo. Jaribio la uthabiti limeendeshwa
  kwa dakika mbili dhidi ya mfumo wa onyesho; hakuna uendeshaji mrefu wala kipimo cha msingi kwenye maunzi yanayofanana na ya
  uzalishaji.
- **Uadilifu wa toleo** (85%): Taswira za toleo zina usanifu mwingi, zimetiwa saini na kuthibitishwa, na saini zake
  zinathibitika. Actions na taswira zimebanwa kwa digest, na historia hukaguliwa kwa siri. Nusu ya amd64 ya taswira ya toleo
  iliyovutwa haijakaguliwa, na bado hakuna mwenyeji wa pili wa msimbo (CRM-T192).
- **Ujumuishaji endelevu** (100%): Hati moja huendesha ukaguzi wote kwenye GitHub Actions na Woodpecker. Misingi ya Rust yote
  miwili na UI ina kiwango cha chini cha 95% cha mistari; milango ya kivinjari, lint, mnyororo wa ugavi, majaribio ya mutation kwenye mistari
  iliyobadilika na majaribio kamili ya mutation ya kila wiki ipo (CRM-T94).
- **Ulinzi wa onyesho** (100%): Kila mgeni wa ziara hupata nakala tofauti ya data ya onyesho, hadi nakala 200. Ziara imezimwa
  kwa chaguo-msingi; weka `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` ili kuitoa (CRM-T92).

## Jinsi ukurasa huu unavyotunzwa

Mlango unapofungwa, badilisha makadirio na ushahidi wake katika commit ile ile ya kazi, na unganisha kwenye kazi hiyo.
