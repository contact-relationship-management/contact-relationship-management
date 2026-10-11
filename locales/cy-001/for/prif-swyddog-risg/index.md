# I'r Prif Swyddog Risg

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Sefyllfa risg

**At ei gilydd: ddim yn barod ar gyfer data a reoleiddir.** Mae'r feddalwedd wedi'i rheoli'n dda ar gyfer ei chyfnod, ond nid yw'r prif risgiau gweddilliol wedi'u lliniaru gan brofion a phroses na all ond y sefydliad sy'n ei mabwysiadu neu bartïon allanol eu darparu.

## Y prif risgiau

| # | Risg | Sail y sgôr | Lliniaru yn yr ystorfa | Gweddilliol |
|---|---|---|---|---|
| 1 | Gwendidau heb eu darganfod | Dim prawf treiddio annibynnol (T42) | Profion ein hunain, gatiau cadwyn gyflenwi, penawdau/CSP llym | **Uchel** nes ei brofi |
| 2 | Datgelu meysydd sensitif i'r rôl anghywir | Rheolau mynediad cymhleth | Cuddio ar sail rôl ar gofnodion, cyfansymiau a mewnwelediadau; prawf matrics personâu; gollyngiad a ganfuwyd ac a drwsiwyd ym manylion y cyfrif (T47) | Canolig: mae'r cuddio wedi'i brofi, heb ei wirio'n annibynnol |
| 3 | Camddefnyddio mynediad brys gan bobl fewnol | Mae torri'r gwydr yn dyrchafu darllen | Rheswm ysgrifenedig, uchafswm o 60 munud, darllen yn unig, adolygiad gorfodol gan ail berson, archwiliedig | Canolig: dim rhybudd wedi'i gysylltu; dim dirymu |
| 4 | Newid llwybr archwilio | Angen mynediad i'r gronfa ddata | Cadwyn hash a sbardun atodi-yn-unig; yn adrodd y toriad cyntaf | Isel-Canolig: rhaid angori hash y pen y tu allan i'r gronfa ddata |
| 5 | Marchnata anghyfreithlon | Gwallau cydsyniad | Mae'r llwybr anfon (nid y rhyngwyneb) yn gorfodi cydsyniad; hanes cydsyniad na ellir ei newid | Canolig nes bod anfonwr go iawn a thrin bownsys a chwynion yn bodoli (T39) |
| 6 | Argaeledd / camddefnydd | API cyhoeddus | Cyfyngu cyfradd a chloi allan, a rennir drwy Redis | Canolig: mae terfynau'n dychwelyd i fesul replica os bydd Redis yn methu |
| 7 | Diffyg cydymffurfio â hygyrchedd | Dyletswydd y sector cyhoeddus | Sganiau axe awtomataidd; datganiad hygyrchedd gonest | **Heb ei fesur** nes archwiliad â llaw (T48) |
| 8 | Cadwyn gyflenwi | Dibyniaethau a delweddau | `cargo deny/audit`, `pnpm audit`, SBOM, pinio crynodebau a adnewyddir gan Dependabot, llif gwaith rhyddhau â llofnodi di-allwedd | Canolig: nid yw llofnodi wedi rhedeg ar ryddhad go iawn (T49) |
| 9 | Cofnodion dyblyg o ailgeisiadau | Mae allweddi idempotency yn ddewisol ar lefel yr API (T52) | Ailchwarae ar gyfer cleientiaid sy'n anfon yr allwedd; mae'r rhyngwyneb cyfun bob amser yn gwneud | Isel |
| 10 | Camddosbarthu data | Mae'r defnyddiwr sy'n lleoli yn penderfynu beth yw “cyswllt” | Nodyn cwmpas a thempled DPIA | Mae'n dibynnu arnoch chi |

## Bylchau sy'n aros ar agor wrth natur

T39 (darparwr e-bost go iawn), T42 (sicrwydd annibynnol), T48 (archwiliad hygyrchedd â llaw), T49 (rhyddhad cyntaf wedi'i lofnodi, adolygydd yn gwirio drafft yr API). Gweler [tasks.md](../../../../spec/tasks.md) a [plan.md](../../../../spec/plan.md).

## Camau risg a awgrymir

Cofrestrwch risgiau 1, 7 a 10 yn ffurfiol; mynnwch T42 cyn unrhyw ddata go iawn; penodwch berchennog ar gyfer adolygu a rhybuddio torri'r gwydr; pennwch pa mor aml i angori pen yr archwiliad.
