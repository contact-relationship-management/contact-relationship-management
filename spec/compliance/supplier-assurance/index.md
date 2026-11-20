# Supplier assurance pack

*Started 2026-11-20 (CRM-T276, for CRM-R99 and CRM-R110).* For an organization that hosts the CRM for a customer and
supports it. That organization is then the customer's processor, and the customer assesses it, not the open-source
project. This pack describes **the software as shipped**. It is not a sign-off, a certification or legal advice, and no
one outside the project has reviewed it.

NHS Wales is the worked example in [CRM-R99](../../requirements.md#crm-r99--a-supplier-assurance-pack). Every scheme it
names (UK GDPR Art. 28, the Welsh Information Governance Toolkit, Cyber Essentials Plus, ISO/IEC 27001, BPSS) is
*not yet read* here, and each has an equivalent in other regions ([regulatory.md](../../regulatory.md)).

| Document | What it gives the customer's assessors |
| --- | --- |
| [Data flow](data-flow.md) | Where personal data is held, every connection the software can make, and what crosses each one |
| [Shared responsibility](shared-responsibility.md) | Who does what for each control area: the software, the hosting supplier, the customer |
| [Sub-processors](sub-processors.md) | A register template for the supplier to fill |
| [Exit and retention](exit-and-retention.md) | What the software can export and erase today, and what it cannot |
| [Licence](licence.md) | Whether hosting is allowed, and what a hosted service must share |

## Evidence the project already has

Linked, not copied, so the pack does not drift from them.

- [Control mapping](../control-mapping.md): each control, its status and its evidence.
- [Personal-data inventory](../personal-data-inventory.md): every table, its data, its retention class.
- [DPIA](../dpia.md) and [record of processing](../record-of-processing.md): filled in as far as the software goes.
- [Operations runbook](../operations-runbook.md): deployment, keys, backups, logging, incident response.
- [Penetration test scope](../penetration-test-scope.md): the brief for a tester. No test has been done (CRM-T42).
- [Security policy](../../../SECURITY.md): reporting, response targets, disclosure.

## What the supplier must supply

The software cannot meet these. In the worked example the customer asks for each one before go-live:

- a processor agreement, including breach notice to the customer within the contract's hours;
- the customer's supplier assessment (the WIGTK in the example);
- certifications for the hosting and support operation (Cyber Essentials Plus as the example's minimum, ISO/IEC 27001
  preferred) and the cloud provider's own attestations;
- staff vetting and location for anyone with production access (BPSS or DBS and UK-based in the example);
- a tested incident response plan, staffed for the hours the contract names;
- an independent penetration test less than a year old (CRM-R108).

The project holds no certificate of any kind.
