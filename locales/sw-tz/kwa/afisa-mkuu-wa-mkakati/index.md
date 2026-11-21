# Kwa Afisa Mkuu wa Mkakati

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Swali la kimkakati

Hii ni ya nini? Leo ni **CRM ya rejeleo inayotanguliza maelezo ya kiufundi** yenye
safu kamili isiyo ya kawaida ya usalama, faragha na ukaguzi. Si bidhaa yenye
wateja, na haishindani na CRM zilizokomaa
([COMPARISONS.md](../../../../COMPARISONS.md)). Chaguo mbele yako ni
kama kuiweka kama rejeleo, au kuwekeza ili iweze kusambazwa katika
mazingira yaliyodhibitiwa.

## Kipi ni tofauti kweli

| Nguvu | Kwa nini inaweza kujalisha | Imara kiasi gani |
|---|---|---|
| Maelezo ya kiufundi kwanza: maelezo moja huendesha huduma ya Rust na UI ya Svelte ([spec](../../../../spec/index.md)) | Nafuu zaidi kukagua, kuhamisha na kudumisha uthabiti | Halisi; majaribio hurejelea maelezo |
| Masoko yanayotanguliza idhini, yanayotekelezwa katika njia ya utumaji | Hitaji gumu kwa wanunuzi wa afya na sekta ya umma | Halisi, lakini bado hakuna mtumaji halisi (T39) |
| Utoaji alama na makadirio yanayoelezeka, yanayotegemea kanuni, kwa wateja watarajiwa | Wanunuzi wanaopaswa kuhalalisha maamuzi kuhusu watu | Halisi; kwa makusudi hakuna ujifunzaji wa mashine |
| Waasiliani wanachukuliwa kutoka huduma ya utambulisho, hawarudufishwi | Data binafsi kidogo inashikiliwa, eneo dogo la uvunjaji | Halisi; inategemea huduma hiyo kuwepo |
| Ukaguzi unaoonyesha uchezaji, ufichaji kwa jukumu, ufikiaji wa dharura wenye sababu | Udhibiti ambao wanunuzi waliodhibitiwa huuliza kwanza | Umetekelezwa na kujaribiwa; haujathibitishwa kwa uhuru |

## Kipi si tofauti

Upana wa vipengele. Kutoa bei, mikataba, simu, utoaji alama wa ujifunzaji wa mashine na
malipo viko nje ya wigo ([scope](../../../../spec/scope.md)). Yeyote anayechagua
CRM kwa vipengele atachagua nyingine.

## Chaguo

| Chaguo | Maana yake | Gharama na hatari | Ushahidi unaohitajika |
|---|---|---|---|
| A. Kubaki rejeleo | Kuiweka kama maktaba ya kufundishia na mifumo | Ya chini kabisa; thamani ni ya sifa | Hakuna zaidi ya leo |
| B. Soko maalum la sekta iliyodhibitiwa | Kumaliza vipengele vilivyo wazi na kuiuza au kuisaidia kwa taasisi za afya na umma | Ya juu kabisa; inahitaji muuzaji, masharti ya kisheria, majaribio ya nje, uhakikisho endelevu | Jaribio huru (T42), ukaguzi wa ufikivu (T48), barua pepe halisi (T39) |
| C. Vijenzi, si bidhaa | Kutoa mnyororo wa ukaguzi, ufichaji na mifumo ya break-glass kama vipande vinavyoweza kutumika tena | Wastani; inahitaji ufungashaji na nyaraka | Kazi ya kutoa; hakuna uthibitisho mpya |

**Pendekezo: fanya A sasa, na ufadhili jaribio la bei nafuu zaidi la B kabla ya kujitolea.**
Tathmini ya NHS ilihitimisha kwamba bidhaa huenda iko nje ya wigo wa DTAC
([tathmini](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
kwa hivyo njia ya kwenda sokoni haijatulia. Tumia kwanza kugundua ni mnunuzi yupi
anahitaji hii na atahitaji nini, si kwa vyeti ambavyo hakuna aliyeviomba.

## Ramani ya njia, kwa mpangilio ambao ushahidi unapendekeza

1. Jaribio huru la usalama na ukaguzi wa mikono wa ufikivu (T42, T48): vinaamua
   kama B inawezekana kabisa.
2. Mtoa huduma halisi wa barua pepe (T39) pindi muuzaji na masharti ya uchakataji wa data
   yanapochaguliwa; kiunganishi cha mtumaji na njia ya SMTP zipo na huendeshwa dhidi ya seva ya majaribio ya ndani, pamoja na kiungo cha kujiondoa kilichotiwa saini, orodha ya kuzuia na webhook ya jumla iliyotiwa saini ya matokeo, lakini hakuna mtoa huduma aliyeunganishwa bado ([roadmap](../../../../spec/roadmap.md)).
3. Thibitisha utiaji saini wa toleo kwa toleo la kwanza lenye alama (T49): hitaji la msingi la
   manunuzi. Mtiririko umejengwa; haujaendeshwa.
4. Kisha tu vipengele vya bidhaa kwenye ramani ya njia (uhusishaji, maeneo,
   saa za SLA za saa za kazi, tovuti ya umma ya hazina ya maarifa).

## Hatari za kimkakati

- **Kudai zaidi ya ukweli.** Njia ya haraka zaidi ya kupoteza mnunuzi aliyedhibitiwa ni
  dai la uzingatiaji ambalo ushahidi hauliungi mkono. Tazama [muhtasari wa GRC](../utawala-hatari-uzingatiaji/index.md).
- **Matengenezo.** Kazi ya usalama haiishi: masasisho ya tegemezi, usasishaji wa
  digest, kuzungusha funguo, kujaribu upya. Chaguo B linakufunga nayo.
- **Kutegemea huduma ya utambulisho** iliyo nje ya hazina hii.
- **Mtunzaji mmoja.** Mradi una mtunzaji mmoja na unasema kwamba kina cha
  ukaguzi ni cha mtu mmoja ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Chaguo B linahitaji zaidi ya mtu mmoja
  anayeweza kuwajibika kwa kazi ya usalama.

## Maamuzi kwako

1. Lipi kati ya A, B au C, na tarehe ya kukagua chaguo hilo.
2. Nani ni mnunuzi wa kwanza lengwa, anayepatikana kwa mazungumzo kabla ya kutumia gharama.
3. Kama utafadhili uhakikisho huru sasa (tazama [muhtasari wa CEO](../afisa-mkuu-mtendaji/index.md)).
