# Llywodraethu, Risg, Cydymffurfiaeth

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Ble i ddechrau

Y pecyn tystiolaeth yw [spec/compliance](../../../../spec/compliance/index.md). Ei reol sylfaenol yw'r un i ddal y tîm ati: mae rheolaeth **Wedi'i gweithredu** dim ond pan fydd cod neu gyflunio yn ei gorfodi a phrawf neu wiriad yn ei phinio; **Defnyddiwr sy'n lleoli** pan fydd yn rhaid i'ch sefydliad ei darparu; **Bwlch** pan nad yw'r naill na'r llall yn wir heddiw.

| Dogfen | Defnyddiwch hi ar gyfer |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, Rheol Ddiogelwch HIPAA, Atodiad A ISO 27001, Erthygl 32 GDPR, pob un wedi'i nodi'n Wedi'i gweithredu, Defnyddiwr sy'n lleoli neu Fwlch |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Asesiad effaith diogelu data, wedi'i ragllenwi â llifoedd data'r system hon |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Caledu, wrth gefn ac adfer, allweddi, logio, ymateb i ddigwyddiadau |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Datganiadau DTAC drafft y GIG i'r gwneuthurwr eu mabwysiadu |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Hunanadolygiad yn erbyn safonau API llywodraeth y DU (drafft, heb ei wirio yn erbyn y testun cyhoeddedig) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Ffurf gwallau (RFC 9457), beth sy'n newid torri, y camau dibrisio a'r cyfnod rhybudd |
| [Asesiad DTAC v2.0 y GIG](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Canlyniad cwestiwn wrth gwestiwn: yn debygol y tu allan i gwmpas DTAC, ac y byddai'n methu'r adrannau perthnasol oherwydd diffyg tystiolaeth sefydliadol |
| [plan](../../../../spec/plan.md) | Dyfarniad parodrwydd (ddim yn barod) a'r gofrestr bylchau |
| [tasks](../../../../spec/tasks.md) | Tasg pob bwlch, ei datrysiad a'r cyfyngiadau a nodwyd |

## Llywodraethu

- **Hawliau penderfynu:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md), [CODEOWNERS](../../../../CODEOWNERS). Mae rheolau cyfraniadau a chymorth AI yn [CONTRIBUTING.md](../../../../CONTRIBUTING.md) ac [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Rheoli newid:** y fanyleb yw ffynhonnell y gwirionedd; mae cod a phrofion yn ei dyfynnu. Mae CI yn rhedeg ar ddau ddarparwr, ac mae `scripts/ci-check.sh` yn ei atgynhyrchu. Rheol yn yr ystorfa: peidiwch â gwanhau prawf na gât i wneud i rywbeth basio.
- **Datgelu:** [SECURITY.md](../../../../SECURITY.md) yw'r llwybr adrodd.

## Risg

Mae'r olwg ar risg yn [briff y Prif Swyddog Risg](../prif-swyddog-risg/index.md). Yn fyr: y risgiau gweddilliol uchaf yw diogelwch heb ei brofi gan bobl o'r tu allan (T42), hygyrchedd heb ei fesur (T48), a dosbarthu data, sy'n benderfyniad i'r sawl sy'n mabwysiadu.

## Cydymffurfiaeth: beth sy'n real a beth sydd ddim

| Maes | Sefyllfa |
|---|---|
| Rheoli mynediad | Wedi'i weithredu a'i brofi: cuddio ar sail rôl, gwiriadau ar lefel cofnod, darlleniadau gweithredol i weinyddwyr yn unig, torri'r gwydr gydag adolygiad gorfodol gan ail berson |
| Archwilio | Wedi'i weithredu a'i brofi: cadwyn hash, atodi-yn-unig, o fewn y trafodyn, archwilio darlleniadau gan gynnwys rhestrau, rhestrau bargeinion a mewnwelediadau. **Defnyddiwr sy'n lleoli:** angori pen y gadwyn y tu allan i'r gronfa ddata |
| Hawliau preifatrwydd | Wedi'i weithredu: allforio mynediad at ddata personol, dileu (a wrthodir tra bo rhwymedigaethau ar agor), ysgubo cadw. **Defnyddiwr sy'n lleoli:** sail gyfreithlon, amserlen gadw, cofnodion prosesu |
| Amgryptio | Yn rhannol: AES-256-GCM ar lefel maes ar ddau golofn gydag allweddi lleol, dim KMS. Amgryptio wrth orffwys ac wrth drosglwyddo yw cyfrifoldeb y defnyddiwr sy'n lleoli |
| Cadwyn gyflenwi | Wedi'i weithredu: `cargo audit`, `cargo deny`, `pnpm audit`, SBOMau, delweddau sylfaen wedi'u pinio yn ôl crynodeb, diweddariadau Dependabot. **Bwlch:** nid yw'r llif gwaith rhyddhau wedi'i lofnodi wedi rhedeg ar ryddhad go iawn (T49) |
| Hygyrchedd | Yn rhannol: sganiau axe awtomataidd a datganiad cyhoeddus gonest. **Bwlch:** dim archwiliad â llaw (T48), cyfieithiadau heb eu hadolygu |
| Sicrwydd annibynnol | **Bwlch:** dim (T42) |
| Cyfraith e-bost a marchnata | Llwybr anfon sy'n rhoi cydsyniad yn gyntaf wedi'i weithredu; dim ond fel llwybr SMTP ar gyfer datblygu y mae anfon go iawn yn bodoli; mae'r ddolen ddad-danysgrifio, y rhestr atal a webhook canlyniadau wedi'i lofnodi yn bodoli, ond nid oes darparwr wedi'i integreiddio ac mae'r cytundeb prosesydd a'r adolygiad cyfreithiol yn agored (T39) |

## Rheolaethau sydd angen perchennog yn eich sefydliad

1. Adolygu a rhybuddio ar gyfer caniatadau torri'r gwydr (rhybuddiwch ar `break_glass_granted`).
2. Angori pen y gadwyn archwilio a'i wirio yn ôl amserlen (`GET /api/audits/verify`).
3. Tystiolaeth wrth gefn ac adfer, ac ymarfer cylchdroi allweddi.
4. Trin dechreuwyr, symudwyr a gadawyr yn eich darparwr hunaniaeth.
5. Cadw ar gyfer y tabl archwilio, sy'n tyfu gyda thraffig darllen.
6. Diogelu Redis fel y gronfa ddata: mae'n dal data mewnwelediadau a storiwyd dros dro.
7. Cyfuno diweddariadau Dependabot, a gwirio rhyddhad wedi'i lofnodi cyn ei ddefnyddio.

## Beth i wrthod ei lofnodi

Unrhyw ddatganiad bod y system yn cydymffurfio, wedi'i hardystio, yn cydymffurfio â HIPAA neu wedi'i chymeradwyo gan y GIG. Mae'r datganiad amddiffynadwy yn gulach: mae rheolaethau wedi'u mapio, mae'r rhai a weithredwyd wedi'u profi, ac mae'r bylchau wedi'u henwi a'u holrhain.
