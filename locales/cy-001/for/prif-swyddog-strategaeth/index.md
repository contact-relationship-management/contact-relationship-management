# I'r Prif Swyddog Strategaeth

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Y cwestiwn strategol

At beth mae hyn? Heddiw mae'n **CRM cyfeirio sy'n rhoi'r fanyleb yn gyntaf**, gyda haen diogelwch, preifatrwydd ac archwilio anarferol o gyflawn. Nid yw'n gynnyrch â chwsmeriaid, ac nid yw'n cystadlu â CRMau aeddfed ([COMPARISONS.md](../../../../COMPARISONS.md)). Y dewis o'ch blaen yw a ddylid ei gadw fel cyfeirnod, neu fuddsoddi i'w wneud yn ddefnyddiadwy mewn lleoliadau a reoleiddir.

## Beth sy'n wirioneddol wahanol

| Cryfder | Pam y gallai fod yn bwysig | Pa mor gadarn yw |
|---|---|---|
| Manyleb yn gyntaf: mae un fanyleb yn gyrru'r gwasanaeth Rust a'r rhyngwyneb Svelte ([spec](../../../../spec/index.md)) | Rhatach i'w archwilio, ei borthi a'i gadw'n gyson | Go iawn; mae profion yn dyfynnu'r fanyleb |
| Marchnata sy'n rhoi cydsyniad yn gyntaf, wedi'i orfodi yn y llwybr anfon | Gofyniad llym i brynwyr iechyd a'r sector cyhoeddus | Go iawn, ond nid oes anfonwr go iawn eto (T39) |
| Sgorio a rhagweld arweinwyr y gellir eu hegluro, ar sail rheolau | Prynwyr sy'n gorfod cyfiawnhau penderfyniadau am bobl | Go iawn; dim dysgu peirianyddol yn fwriadol |
| Cysylltiadau'n cael eu defnyddio o wasanaeth hunaniaeth, heb eu dyblygu | Llai o ddata personol yn cael ei ddal, arwyneb tor-diogelwch llai | Go iawn; mae'n dibynnu ar fodolaeth y gwasanaeth hwnnw |
| Archwilio sy'n dangos ymyrraeth, cuddio yn ôl rôl, mynediad brys â rheswm | Y rheolaethau y mae prynwyr a reoleiddir yn gofyn amdanynt gyntaf | Wedi'u gweithredu a'u profi; heb eu gwirio'n annibynnol |

## Beth sydd ddim yn wahanol

Ehangder nodweddion. Mae dyfynbrisiau, contractau, teleffoni, sgorio dysgu peirianyddol a thaliadau y tu allan i'r cwmpas ([scope](../../../../spec/scope.md)). Bydd unrhyw un sy'n dewis CRM ar sail nodweddion yn dewis un arall.

## Opsiynau

| Opsiwn | Beth mae'n ei olygu | Cost a risg | Tystiolaeth sydd ei hangen |
|---|---|---|---|
| A. Aros yn gyfeirnod | Ei gadw fel llyfrgell addysgu a phatrymau | Isaf; mae'r gwerth yn enw da | Dim y tu hwnt i heddiw |
| B. Cilfach sector a reoleiddir | Gorffen yr eitemau agored a'i werthu neu ei gefnogi i gyrff iechyd a chyhoeddus | Uchaf; angen gwerthwr, telerau cyfreithiol, profi allanol, sicrwydd parhaus | Prawf annibynnol (T42), archwiliad hygyrchedd (T48), e-bost go iawn (T39) |
| C. Cydrannau, nid cynnyrch | Cynnig y gadwyn archwilio, cuddio a phatrymau torri'r gwydr fel darnau y gellir eu hailddefnyddio | Canolig; angen pecynnu a dogfennaeth | Gwaith echdynnu; dim ardystiad newydd |

**Argymhelliad: gwnewch A nawr, ac ariannwch y prawf rhataf o B cyn ymrwymo.** Daeth asesiad y GIG i'r casgliad ei bod yn debygol nad yw'r cynnyrch o fewn cwmpas DTAC ([asesiad](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)), felly nid yw'r llwybr i'r farchnad wedi'i setlo. Gwariwch yn gyntaf ar ddarganfod pa brynwr sydd angen hyn a beth fyddai ei angen arno, nid ar ardystiadau na ofynnodd neb amdanynt.

## Y map ffordd, yn y drefn y mae'r dystiolaeth yn ei hawgrymu

1. Prawf diogelwch annibynnol ac archwiliad hygyrchedd â llaw (T42, T48): maent yn penderfynu a yw B yn bosibl o gwbl.
2. Darparwr e-bost go iawn (T39); mae haen anfon a llwybr SMTP yn bodoli ac yn rhedeg yn erbyn gweinydd prawf lleol, gyda dolen ddad-danysgrifio wedi'i llofnodi, rhestr atal a webhook canlyniadau generig wedi'i lofnodi, ond heb ddarparwr wedi'i integreiddio eto, unwaith y dewisir gwerthwr a thelerau prosesu data ([roadmap](../../../../spec/roadmap.md)).
3. Profi llofnodi rhyddhau gyda rhyddhad cyntaf wedi'i dagio (T49): gofynion sylfaenol ar gyfer caffael. Mae'r llif gwaith wedi'i adeiladu; nid yw wedi rhedeg.
4. Dim ond wedyn y nodweddion cynnyrch ar y map ffordd (priodoli, tiriogaethau, clociau SLA oriau busnes, porth sylfaen wybodaeth gyhoeddus).

## Risgiau strategol

- **Honni mwy nag sy'n wir.** Y ffordd gyflymaf o golli prynwr a reoleiddir yw hawliad cydymffurfio nad yw'r dystiolaeth yn ei gefnogi. Gweler [briff Llywodraethu, Risg, Cydymffurfiaeth](../llywodraethu-risg-cydymffurfiaeth/index.md).
- **Cynnal a chadw.** Nid yw gwaith diogelwch byth yn dod i ben: diweddaru dibyniaethau, adnewyddu crynodebau, cylchdroi allweddi, ail-brofi. Mae opsiwn B yn eich ymrwymo i hynny.
- **Dibyniaeth ar wasanaeth hunaniaeth** y tu allan i'r ystorfa hon.
- **Un cynhaliwr.** Mae gan y prosiect un cynhaliwr ac mae'n dweud mai dyfnder un person yw dyfnder yr adolygu ([MAINTAINERS.md](../../../../MAINTAINERS.md), [GOVERNANCE.md](../../../../GOVERNANCE.md)). Mae opsiwn B angen mwy nag un person all ateb dros y gwaith diogelwch.

## Penderfyniadau i chi

1. Pa un o A, B neu C, a'r dyddiad adolygu ar gyfer y dewis hwnnw.
2. Pwy yw'r prynwr targed cyntaf, wedi'i ganfod drwy sgwrs cyn gwario.
3. A ddylid ariannu sicrwydd annibynnol nawr (gweler [briff y Prif Swyddog Gweithredol](../prif-swyddog-gweithredol/index.md)).
