# Y llwybr i gynhyrchu

> Nid yw Rheoli Perthnasoedd Cyswllt yn barod ar gyfer cynhyrchu. Mae'r dudalen hon yn esbonio beth yw ystyr
> cynhyrchu yma, a pha mor bell y mae pob rhan wedi dod.

Mae'r feddalwedd yn rhedeg ar ddata synthetig. Caiff e-bost ei efelychu oni bai bod modd post go iawn wedi'i
ffurfweddu. Peidiwch â rhoi data personol go iawn ynddi.

## Beth yw ystyr cynhyrchu

Mae defnydd cynhyrchu yn cadw data pobl go iawn ac yn anfon negeseuon go iawn atynt. Ar gyfer y prosiect hwn mae
angen y cyfan o'r canlynol:

- **Mewngofnodi go iawn.** Caiff pob cais ei ddilysu, ac mae'r rheolau mynediad yn cyfateb i rolau'r sefydliad sy'n
  ei lleoli.
- **Anfon cyfreithlon.** Mae gan bob neges ganiatâd wedi'i gofnodi a sail gyfreithlon dan gyfraith pob man y caiff ei
  hanfon iddo, ac mae'n mynd drwy ddarparwr e-bost go iawn sy'n trin negeseuon a wrthodwyd, cwynion a datdanysgrifio.
- **Sicrwydd annibynnol.** Mae pobl y tu allan i'r prosiect wedi profi'r diogelwch a'r hygyrchedd, ac mae'r
  canfyddiadau wedi'u trwsio neu eu derbyn.
- **Gweithrediadau.** Copïau wrth gefn sydd wedi'u hadfer, monitro, proses digwyddiadau, a rhywun ar alwad.
- **Tystiolaeth o sefydlogrwydd.** Rhediadau hir dan lwyth realistig, ar galedwedd tebyg i'r defnydd, heb wallau na
  arafu.

Penderfynir ar barodrwydd ar gyfer cynhyrchu fesul defnydd. Dim ond ar gyfer ei sefydliad a'i dderbynwyr ei hun y
gall lleolwr gau rhai gatiau. Yr asesiad peirianyddol y tu ôl i'r dudalen hon yw
[spec/plan.md](../../../spec/plan.md). Mae'r gatiau agored yn [spec/tasks.md](../../../spec/tasks.md).

## Parodrwydd

Amcangyfrif a wnaed ar 2026-11-13 o'r rhestr dasgau yw pob canran, nid mesuriad.

- **Dilysu a rheoli mynediad** (70%): Mae mewngofnodi drwy docynnau PASETO neu Keycloak/OIDC, rheolau mynediad ar sail
  priodoleddau gyda gwiriadau ar lefel cofnod, a modd cynhyrchu sy'n gwrthod cychwyn heb ddilysu wedi'u hadeiladu.
  Nid oes unrhyw ddefnydd go iawn wedi'u troi ymlaen gyda'i briodoleddau ei hun, ac nid yw'r matrics gorfodi eto'n
  cwmpasu pob triniwr (CRM-G1).
- **Danfon e-bost** (40%): Anfonwr efelychiedig yw'r rhagosodiad, ac mae anfonwr SMTP yn bodoli. Nid oes addasydd
  darparwr e-bost gyda bachau gwe ar gyfer negeseuon a wrthodwyd a chwynion na rhestr atal (CRM-T39).
- **Cyfraith diogelu data ac e-farchnata** (15%): Mae cofnodion caniatâd, mynediad at ddata, dileu a chadw wedi'u
  hadeiladu. Nid oes cyfraith unrhyw ranbarth wedi'i darllen ar gyfer y prosiect hwn, ac mae'r adolygiad o'r llwybr
  anfon go iawn ar agor (CRM-G2, [regulatory](../../../spec/regulatory.md)).
- **Sicrwydd diogelwch annibynnol** (10%): Mae cwmpas prawf treiddio wedi'i ysgrifennu. Nid oes prawf treiddio,
  dadansoddiad risg ffurfiol nac achrediad wedi'u gwneud (CRM-T42). Nid yw'r archwiliadau diogelwch AI yn sicrwydd
  annibynnol.
- **Hygyrchedd** (50%): Mae gwiriadau awtomatig axe-core yn erbyn WCAG 2.2 A ac AA yn pasio ar bob llwybr tudalen. Nid
  oes archwiliad â llaw gyda bysellfwrdd yn unig, darllenwyr sgrin na chwyddo 400% wedi'i wneud (CRM-T48).
- **Cyfieithiadau** (30%): Mae'r rhyngwyneb ar gael mewn wyth iaith. Peiriant a ysgrifennodd y llinynnau mewn
  ieithoedd heblaw Saesneg, ac nid oes siaradwr wedi'u hadolygu.
- **Gweithrediadau** (30%): Mae llyfr gweithrediadau yn cwmpasu copïau wrth gefn ac adfer, rheoli allweddi, amgryptio
  wrth storio, ymateb i ddigwyddiadau a chadw logiau. Nid oes adferiad wedi'i brofi, ac mae rhestr wirio mynd yn fyw y
  llyfr ar agor.
- **Perfformiad a sefydlogrwydd** (20%): Mae profion llwyth a phrawf sefydlogrwydd hirdymor yn bodoli. Mae'r prawf
  sefydlogrwydd wedi rhedeg am ddau funud yn erbyn y pentwr arddangos; nid oes rhediad hir na llinell sylfaen ar
  galedwedd tebyg i gynhyrchu.
- **Uniondeb rhyddhau** (85%): Mae delweddau rhyddhau yn aml-saernïaeth, wedi'u llofnodi a'u hardystio, ac mae eu
  llofnodion yn gwirio. Mae camau gweithredu a delweddau wedi'u pinio wrth eu crynhoad, a chaiff yr hanes ei sganio
  am gyfrinachau. Nid yw hanner amd64 delwedd ryddhau a dynnwyd wedi'i wirio, ac nid oes ail westeiwr cod eto
  (CRM-T192).
- **Integreiddio parhaus** (60%): Mae un sgript yn rhedeg pob gwiriad, ar GitHub Actions a Woodpecker. Mae lloriau
  cwmpas, profi mwtaniadau wedi'i amserlennu a gatiau eraill ar agor o hyd (CRM-T94).
- **Mesurau diogelu'r arddangosiad** (60%): Mae pob ymwelydd â'r daith yn cael copi ar wahân o'r data arddangos, hyd
  at uchafswm o 200 copi. Mae'r daith ymlaen o hyd oni bai bod `CRM_DEMO_TOUR=off` wedi'i osod (CRM-T92).

## Sut y cedwir y dudalen hon

Pan fydd giât yn cau, newidiwch yr amcangyfrif a'i dystiolaeth yn yr un ymrwymiad â'r gwaith, a chysylltwch y dasg.
