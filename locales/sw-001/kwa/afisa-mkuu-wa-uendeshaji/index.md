# Kwa Afisa Mkuu wa Uendeshaji

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Kinachohitajika kuiendesha

Huduma ni mchakato mmoja wa Rust kwa kila nakala pamoja na PostgreSQL 18 na Redis; sehemu ya mbele
ni mchakato wa Node. Zote mbili husafirishwa kama taswira za kontena zilizoimarishwa, zisizo za root,
zenye taswira za msingi zilizobanwa kwa digest. Mwongozo wa uendeshaji ni
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Ukweli wa uendeshaji wa kupangia

- **Hushindwa kwa usalama wakati wa kuanza.** Katika uzalishaji huduma hukataa kuanza ikiwa
  uthibitishaji umezimwa, ikiwa mnyororo wa ukaguzi hautumiki, au ikiwa funguo zinazohitajika na
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` hazipo. Usambazaji uliowekwa vibaya husimama badala ya kuendesha wazi.
- **Afya na vipimo.** `/_health/live`, `/_health/ready`, `/_health/startup` hazizuiwi kamwe
  kwa kikomo cha kasi, kwa hivyo kufungiwa hakuwezi kuanzisha upya pod yenye afya. Vipimo kwenye
  `/metrics.prom`.
- **Vikomo vya matumizi mabaya** vimewashwa katika uzalishaji: maombi 600 kwa dakika kwa kila mteja na
  kufungiwa baada ya kushindwa kuingia mara 10 ndani ya dakika 5. Vinashirikiwa katika
  nakala zote kupitia Redis; Redis ikishindwa hurudi kwenye hesabu kwa kila nakala.
- **Ufikiaji wa dharura** (break-glass): mtumiaji yeyote aliyeingia anaweza kuchukua
  kupandishwa daraja kwa usomaji wenye sababu hadi dakika 60. Mtu mwingine lazima akague kila ruhusa.
  **Mchakato lazima uendeshe:** toa tahadhari kwa safu za ukaguzi za `break_glass_granted` na
  kagua idadi ya ambazo hazijakaguliwa (ukurasa wa msimamizi `/break-glass/review`).
- **Makosa** yana umbo moja (RFC 9457 `application/problem+json`, na `error` na
  `description` zimehifadhiwa), kwa hivyo uchanganuzi wa kumbukumbu na kanuni za tahadhari zinaweza kutegemea `status` na `type`.
- **Ukuaji wa ukaguzi.** Usomaji hukaguliwa, kwa hivyo jedwali la ukaguzi hukua kulingana na trafiki.
  Panga uhifadhi wake. Mteja anayepiga mara kwa mara atalikuza haraka.

## Kazi ya uendeshaji iliyo wazi

| Kipengele | Hali |
|---|---|
| Mazoezi ya nakala rudufu/urejeshaji, mazoezi ya kuzungusha funguo | Imeandikwa kwenye mwongozo; **wewe** lazima uyaendeshe na kuyathibitisha |
| Kutia nanga kichwa cha mnyororo wa ukaguzi nje ya hifadhidata | Wajibu wako; API inaripoti hash ya kichwa |
| Kusasisha digest za taswira zilizobanwa | Dependabot hufungua maombi ya kuunganisha kila wiki; **wewe** lazima uyakague na kuyaunganisha la sivyo kubanwa kunapitwa na wakati |
| Kuthibitisha toleo lililotiwa saini kabla ya kulisambaza | Amri ziko kwenye mwongozo; sambaza kwa digest, na ukatae taswira inayoshindwa uthibitisho. Haijajaribiwa hadi toleo la kwanza lenye alama |
| Mtoa huduma halisi wa barua pepe (T39) | Wazi; hubadilisha picha ya matukio na wasambazaji |
| Redis inashikilia data ya uchanganuzi | Ilinde kama hifadhidata: TLS, nenosiri, isifichuliwe hadharani |

## Vikomo vinavyojulikana vinavyostahili kuambiwa timu ya zamu

Kutoka kiotomatiki kwa kutofanya kazi hufanyika upande wa mteja; tokeni iliyofutwa hukataliwa mara moja, lakini
upakiaji upya wa sera huonekana katika uchanganuzi ulio kwenye akiba hadi sekunde 30; break-glass haiwezi
kufutwa, inaisha muda tu.
