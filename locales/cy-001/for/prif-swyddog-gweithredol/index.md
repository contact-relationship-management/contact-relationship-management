# I'r Prif Swyddog Gweithredol

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Y penderfyniad mewn un paragraff

System CRM lefel arddangos yw hon, gyda sylfaen diogelwch a phreifatrwydd anarferol o gryf. Nid yw **yn barod** i ddal data go iawn am gleifion na dinasyddion, ac mae'r ystorfa ei hun yn dweud hynny ([plan.md](../../../../spec/plan.md)). Penderfyniad i ariannu'r gwaith allanol sy'n weddill yw ei mabwysiadu, nid penderfyniad i'w throi ymlaen.

## Beth sy'n bodoli

- CRM gweithredol ar draws gwerthiant, marchnata, cymorth a dadansoddeg, gyda API Rust a pharth blaen gwe, wedi'i adeiladu yn ôl manyleb ysgrifenedig.
- Marchnata sy'n rhoi cydsyniad yn gyntaf; rheolaeth mynediad sy'n cuddio gwerthoedd sensitif rhag rolau na ddylent eu gweld; llwybr archwilio sy'n dangos ymyrraeth; mynediad at ddata personol a dileu ar gyfer ceisiadau preifatrwydd; a mynediad brys (“torri'r gwydr”) sydd â rheswm, cyfyngiad amser ac adolygiad.
- Mae 48 o'r 53 eitem waith a olrheinir wedi'u cwblhau ([tasks.md](../../../../spec/tasks.md)).

## Beth sy'n sefyll rhwng hyn a defnydd go iawn

| Rhwystr | Pam mae'n bwysig | Pwy all ei ddatrys |
|---|---|---|
| Dim danfon e-bost go iawn (T39) | Efelychiad yw'r marchnata; mae darparwr go iawn angen contractau a chytundeb prosesu data | Chi sy'n dewis y gwerthwr; y tîm cyfreithiol sy'n llofnodi |
| Dim prawf diogelwch annibynnol (T42) | Mae ysbytai a phrynwyr y llywodraeth fel arfer yn gofyn amdano; nid yw ein profion ein hunain yn annibynnol | Cwmni allanol |
| Dim archwiliad hygyrchedd â llaw (T48) | Rhaid i gyrff cyhoeddus gyhoeddi datganiad hygyrchedd y gallant sefyll y tu ôl iddo | Pobl â thechnoleg gynorthwyol |
| Adolygiad llofnodi adeiladau a safonau API heb ei orffen (T49) | Mae rhestrau gwirio caffael yn gofyn am y ddau | Peirianneg, gyda phenderfyniad ar ddalfa allweddi |
| Diffyg allweddi idempotency (T52) | Gall cais creu a ailgeisir ddyblygu cofnod | Peirianneg |

## Penderfyniadau sy'n eiddo i chi

1. **Pa farchnad yn gyntaf.** Mae asesiad Meini Prawf Asesu Technoleg Ddigidol y GIG ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)) yn dod i'r casgliad ei bod yn debygol nad yw'r cynnyrch o fewn cwmpas DTAC ac y byddai'n methu'r adrannau sy'n berthnasol oherwydd diffyg tystiolaeth sefydliadol (swyddog diogelwch clinigol a enwir, ardystiadau). Dewiswch y llwybr cyn gwario.
2. **A ddylid ariannu sicrwydd annibynnol** (T42, T48). Hebddo hynny, yr hawliad gonest yw “wedi'i adeiladu i safon uchel, heb ei brofi gan bobl o'r tu allan”.
3. **Pwy sy'n berchen ar y risg** unwaith y bydd data go iawn yn cael ei ddal. Gweler briffiau'r [Prif Swyddog Risg](../prif-swyddog-risg/index.md) a'r [Prif Swyddog Cyfreithiol](../prif-swyddog-cyfreithiol/index.md).

## Beth i beidio â'i ddweud yn allanol

Peidiwch â honni cydymffurfiaeth, ardystiad, “cydymffurfio â HIPAA” na “chymeradwyo gan y GIG”. Mae'r pecyn tystiolaeth ([compliance](../../../../spec/compliance/index.md)) yn cefnogi datganiad culach a gwir: mae rheolaethau wedi'u mapio, mae'r rhai a weithredwyd wedi'u profi, ac mae bylchau wedi'u henwi.
