# Regulatory posture

> ⚠️ **Demo software.** Not a production CRM; no real personal
> data; no real email is sent (delivery is simulated).

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
  e-privacy rules; other regions not yet read, see [worldwide.md](worldwide.md)):
  lawful basis, disclosure in the privacy notice. It is the operator's to
  settle before turning it on.
- **Auditability** — mutations + consent history + sensitive reads.
- **Synthetic data only** in seeds and tests.

## Production would additionally require

- Compliance with the data-protection and e-marketing law of each
  jurisdiction the deployer sends to, for the real send path (read so far:
  the UK GDPR, the EU GDPR and the UK's PECR; every other region is not yet
  read, see [worldwide.md](worldwide.md)):
  lawful basis per channel, soft opt-in rules where a regime has them, sender
  identification, working unsubscribe in every message; ESP
  contract terms (processor agreements); retention schedules for
  leads/activities/tickets; subject-access and erasure coordinated
  with the identity services; and — if scoring ever influences
  significant decisions — automated-decision-making review.
  Tracked as production gates in [tasks.md](tasks.md).
- Medical-device status, if a deployer gives the product a medical
  purpose: in Great Britain that is a question for the
  [MHRA](https://www.gov.uk/government/organisations/medicines-and-healthcare-products-regulatory-agency). The project as shipped is not a device and has not
  sought the regulator's view (see
  [AI_STATEMENT](special-files-for-public-repos/AI_STATEMENT.md)). A
  manufacturer's risk management under ISO 14971 would start from
  [compliance/risk-management.md](compliance/risk-management.md).

## Production readiness

[plan.md](plan.md) assesses readiness for medical and governmental
organizations and lists the gaps; [compliance/](compliance/index.md) holds the
control mapping, DPIA template, and operations runbook.
