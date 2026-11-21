# Exit and retention

*Written 2026-11-20 (CRM-T276, CRM-R99, CRM-R109).* What the software can do today when a contract ends, and what the
supplier must do by other means.

## What the software gives today

- **One contact's data:** `GET /api/contacts/{pid}/subject-access` is one audited JSON export of a contact and what hangs
  off it (CRM-R20).
- **A table as a file:** each data table in the UI exports its rows as CSV, TSV or Excel, in the browser, from what the
  person can already see (CRM-R21). It is not a whole-deployment export.
- **Retention:** soft-deleted rows of the `swept` tables are hard-deleted by an operator-run sweep after
  `CONTACT_RELATIONSHIP_MANAGEMENT_RETENTION_DAYS`. Which tables are swept, kept as evidence or have no deletion path is in the
  [personal-data inventory](../personal-data-inventory.md).
- **Erasure of one contact:** `POST /api/contacts/{pid}/erase` anonymises the contact (CRM-R20).

## What the supplier must do at exit today

There is no whole-deployment export in the software (CRM-T285). Until there is:

1. Take a consistent `pg_dump` of the customer's database and a copy of the attachment directory; give them to the
   customer encrypted to a key the customer holds, with a manifest of sizes and digests.
2. Give the field keyring as well, or the two sealed columns cannot be read.
3. After the customer confirms the export, destroy the database, its replicas and backups, the attachment store, Redis,
   the logs and the keys, and certify what was destroyed and when.

A `pg_dump` is SQL, one of the formats the worked example accepts, but it is the software's own schema, not a format
agreed with the customer.

## Not built

A whole-deployment export in JSON lines or CSV with a manifest; erasure by key destruction; a certificate template;
retention horizons per class (all CRM-T285).
