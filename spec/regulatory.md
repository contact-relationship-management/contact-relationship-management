# Regulatory posture

> ⚠️ **Demo software.** Not a production CRM; no real personal
> data; no real email is sent (delivery is simulated).

The law is recorded **by region** ([the register by region](#the-register-by-region), at the end): no
jurisdiction is the default, and every region, the UK and the EU included, is *not yet read*. Nothing in
this file is legal advice.

## Observed by design

- **Consent-first marketing** — no send without recorded, current
  consent; unsubscribe is immediate, permanent until re-grant, and
  bypasses nothing (the send path enforces it, not the UI). The
  append-only consent history is the compliance evidence.
- **Data minimisation** — contacts/accounts are URN wrappers;
  demographics stay in the identity registries; CRM holds
  relationship state.
- **Access control** — ABAC personas + amount/channel masking;
  the `CRM_REQUIRE_AUTH` activation gate must be on before real
  exposure.
- **No tracking switched on by the CRM** — the service never asks a mail
  provider to track opens or clicks (the SendGrid request turns both off
  so links stay as written). If an operator turns tracking on at the
  provider, the service records an open or a click only while the
  contact's consent for the campaign's purpose is `granted` and the
  mailbox is not suppressed (CRM-R58). A tracking pixel is a matter of its
  own under the e-privacy and data-protection law where the mail is read
  (in the UK, PECR and the UK GDPR; in the EU, the GDPR and the national
  e-privacy rules; other regions not yet read, see [the register by region](#the-register-by-region)):
  lawful basis, disclosure in the privacy notice. It is the operator's to
  settle before turning it on.
- **Auditability** — mutations + consent history + sensitive reads.
- **Synthetic data only** in seeds and tests.

## Production would additionally require

- Compliance with the data-protection and e-marketing law of each
  jurisdiction the deployer sends to, for the real send path (the worked
  examples so far are the UK GDPR, the EU GDPR and the UK's PECR, written from
  general knowledge and not checked against their text; every region,
  those included, is *not yet read*: see [the register by region](#the-register-by-region)):
  lawful basis per channel, soft opt-in rules where a regime has them, sender
  identification, working unsubscribe in every message; ESP
  contract terms (processor agreements); retention schedules for
  leads/activities/tickets; subject-access and erasure coordinated
  with the identity services; and — if scoring ever influences
  significant decisions — automated-decision-making review.
  Tracked as production gates in [tasks.md](tasks.md).
- Medical-device status, if a deployer gives the product a medical
  purpose: in Great Britain that is a question for the
  [MHRA](https://www.gov.uk/government/organisations/medicines-and-healthcare-products-regulatory-agency);
  elsewhere for the regulator the register names for that region (*not yet read*). The project as shipped is not a device and has not
  sought the regulator's view (see
  [AI_STATEMENT](special-files-for-public-repos/AI_STATEMENT.md)). A
  manufacturer's risk management under ISO 14971 would start from
  [compliance/risk-management.md](compliance/risk-management.md).

## Production readiness

[plan.md](plan.md) assesses readiness for medical and governmental
organizations and lists the gaps; [compliance/](compliance/index.md) holds the
control mapping, DPIA template, and operations runbook.

## The register by region

*Added 2026-10-31 by CRM-T165 (CRM-R95, CRM-D19). Mirrors the workforce planning app's register by region (WPM-T370) and the
project portfolio app's data-protection map (its `spec/12-compliance.md` §12.2a, WW-1.2).*

> **Not legal advice, and nothing here was read.** Every entry below is **not yet read**: the laws are named from general knowledge on
> 2026-10-31, as examples of what a deployer in that region would look at first, and **no law's text, guidance or regulator's page was
> consulted for this register** (no source was opened). Names, numbers and years may be wrong or out of date, and a region's list is
> not complete. A deployer confirms its own obligations with its own advisers. The UK and EU entries are the worked examples the rest of
> the spec was written against; they are *not yet read* too (CRM-T164 recorded that no law was read).

**How to read it.** The CRM names no jurisdiction in its code (CRM-D19). What every row gets alike from the software: consent per
purpose with double opt-in and an append-only history, and a send path that sends only on `granted` (CRM-R6, CRM-R29); a subject-access
export and an erasure path (CRM-R20); retention horizons the deployer sets, whose defaults are no jurisdiction's
(`GET /api/retention` says so in `horizon_basis`); a hash-chained audit log; `List-Unsubscribe` and one-click unsubscribe (RFC 2369,
RFC 8058); no cross-border transfer made by the software itself. What no row gets yet: a consent regime other than opt-in (soft opt-in,
implied consent with an expiry, opt-out), consent per channel, and a sender identification and postal address in each message — the
design is CRM-D21 (confirmed 2026-10-31, CRM-T166). Residency, transfer safeguards, breach notices, registration with a regulator, a local
representative and the privacy notice are the deployer's in every row. How each row maps onto the CRM's controls is
[compliance/control-mapping.md](compliance/control-mapping.md#by-region).

| Region | Data protection (examples) | E-marketing and e-privacy (examples) | Medical-device regulator, if a deployer gives a medical purpose (examples) | Status |
| --- | --- | --- | --- | --- |
| International standards | Council of Europe Convention 108+; the OECD Privacy Guidelines; ISO/IEC 29100 (privacy framework) and ISO/IEC 27701 (privacy information management) | RFC 2369 and RFC 8058 (unsubscribe headers; built in `src/mail.rs`); the large mailbox providers' bulk-sender rules (not law) | ISO 14971 (risk management, mapped in [compliance/risk-management.md](compliance/risk-management.md)); IEC 62304; the IMDRF's software-as-a-medical-device documents | *not yet read* |
| European Union (and the EEA) | GDPR, Regulation (EU) 2016/679 | the ePrivacy Directive 2002/58/EC as each member state transposed it (a supranational layer and a national one) | Regulation (EU) 2017/745 (MDR); the AI Act, Regulation (EU) 2024/1689, if scoring were ever used for decisions about people | *not yet read* |
| United Kingdom | UK GDPR and the Data Protection Act 2018, as amended (the Data (Use and Access) Act 2025) | PECR 2003 (its soft opt-in for existing customers is the shape CRM-D21 calls `soft_opt_in`) | the MHRA, under the UK Medical Devices Regulations 2002 | *not yet read* |
| United States | no general federal law; state laws (for example California's CCPA as amended by the CPRA, and a growing list of other states); HIPAA where the deployer is a covered entity or business associate | the CAN-SPAM Act (an opt-out regime with a postal address in each message, as commonly described); the TCPA for calls and texts; state laws | the FDA | *not yet read* |
| Canada | PIPEDA; provincial laws (for example Québec's Law 25, and Alberta's and British Columbia's private-sector acts) | CASL (express or implied consent, implied consent lapsing after a period, as commonly described: the shape CRM-D21 calls `implied`) | Health Canada | *not yet read* |
| Latin America and the Caribbean | Brazil's LGPD (Law 13.709/2018); Argentina's Law 25.326; Mexico's federal personal-data law; Chile's Law 21.719; Colombia's Law 1581 of 2012; others | rules inside the data-protection laws and consumer laws; national do-not-call registers | ANVISA (Brazil), COFEPRIS (Mexico), ANMAT (Argentina), others | *not yet read* |
| Africa | South Africa's POPIA; Nigeria's NDPA 2023; Kenya's Data Protection Act 2019; Egypt's Law 151 of 2020; others; the African Union's Malabo Convention (a supranational layer) | rules inside the data-protection laws (for example POPIA's section on direct marketing by electronic communication) | SAHPRA (South Africa), NAFDAC (Nigeria), others | *not yet read* |
| Middle East and the Gulf | Saudi Arabia's PDPL; the UAE's federal PDPL, with the DIFC's and ADGM's own laws in those free zones (a sub-national layer with its own law); Qatar's, Bahrain's and Oman's laws; Israel's Privacy Protection Law; Türkiye's KVKK | Türkiye's law on commercial electronic messages and its central consent register; rules inside the data-protection and telecom laws elsewhere | the SFDA (Saudi Arabia), the UAE's health authorities, others | *not yet read* |
| South Asia | India's Digital Personal Data Protection Act 2023 and its rules; Sri Lanka's Personal Data Protection Act 2022; others (Pakistan and Bangladesh not yet identified here) | India's telecom regulator's rules on unsolicited commercial communications (calls and texts) | the CDSCO (India), others | *not yet read* |
| East Asia | China's PIPL; Japan's APPI; South Korea's PIPA; Taiwan's PDPA; Hong Kong's PDPO | Japan's act on specified electronic mail (opt-in, as commonly described); South Korea's Network Act (opt-in and labelling of advertising); Hong Kong's Unsolicited Electronic Messages Ordinance; China's rules on commercial messages (not yet identified here) | the NMPA (China), the PMDA (Japan), the MFDS (South Korea) | *not yet read* |
| South-East Asia | Singapore's PDPA; Malaysia's PDPA 2010; Thailand's PDPA 2019; Indonesia's Law 27 of 2022; the Philippines' Data Privacy Act 2012; Vietnam's personal-data rules; ASEAN's data-management framework (a regional layer, not law) | Singapore's Do Not Call provisions and Spam Control Act; rules inside the data-protection laws elsewhere | Singapore's Health Sciences Authority, others | *not yet read* |
| Oceania | Australia's Privacy Act 1988 (the Australian Privacy Principles); New Zealand's Privacy Act 2020; the Pacific island states not yet identified here | Australia's Spam Act 2003 and New Zealand's Unsolicited Electronic Messages Act 2007 (consent, sender identification and a working unsubscribe, as commonly described) | the TGA (Australia), Medsafe (New Zealand) | *not yet read* |

A row is marked *read* only when the law's text was read for this project, with the date, the source and how much of it, as WPM-T384's
`read`, `unverified`, `example` and `starting_point` statuses say; until then nothing in the spec may say what that law requires. A
federation (the United States, Canada, the UAE with its free zones, the EU with its member states) is more than one layer, and a
contact or a send may be under several (CRM-D19).

**Instead of a GDPR DPIA and record of processing.** The [DPIA](compliance/dpia.md) and the
[record of processing](compliance/record-of-processing.md) are written as the GDPR's (Art. 35 and Art. 30) because that is the worked
example; a deployer under another law uses them as the system facts for the instrument its own law asks for, under that law's name
(for example a privacy impact assessment, or the LGPD's data-protection impact report), and keeps the records its law asks for. Which
instrument each law asks for is *not yet read*.

**Assurance by country or body.** The generic assurance mapping is [compliance/control-mapping.md](compliance/control-mapping.md),
against international or widely used frameworks (ISO/IEC 27001 Annex A, NIST SP 800-53) and, by region, the table above. A scheme that
belongs to one country or one body lives under `docs/<country-or-body>/<scheme>/<version>/`, named in full in lower case with hyphens:
today the only one is `docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/` (the NHS DTAC; its
draft statements are [compliance/dtac-supporting.md](compliance/dtac-supporting.md), and
[compliance/gds-api-review.md](compliance/gds-api-review.md) is the UK government's API standards). Another country's scheme (for
example a national health-software assessment) or a body that spans countries goes in a folder of its own beside it, and none is the
default. No other folder exists yet.

**CRM-G2's scope (answered by the product owner, 2026-10-31).** The production gate CRM-G2 covers the data-protection and
e-marketing law of every jurisdiction the deployment sends to, as the deployer lists them, with the GDPR and PECR as the first
worked case; each deployer passes the gate for its own recipients. The gate's text is in [tasks.md](tasks.md) (CRM-G2).
