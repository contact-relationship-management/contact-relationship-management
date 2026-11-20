# Sub-processor register (template)

*Written 2026-11-20 (CRM-T276, CRM-R99).* For the hosting supplier to fill in and give to the customer. The rows are
every kind of outside service the software can use ([data flow](data-flow.md)); delete a row the deployment does not use,
and add any the supplier uses for itself (support tooling, ticketing, monitoring).

| Kind | Used for | Personal data it receives | Provider | Location of data and of support | Attestations (ISO/IEC 27001, SOC 2, others) | Agreement in place |
| --- | --- | --- | --- | --- | --- | --- |
| Cloud infrastructure | hosts the service, the UI and their network | everything | | | | |
| PostgreSQL (managed or self-run) | every record and the audit trail | everything | | | | |
| Redis | counters, break-glass grants, cached insight answers | cached answers | | | | |
| File storage | attachments | attachments | | | | |
| Backups and recovery copies | copies of the above | everything | | | | |
| Mail provider or SMTP relay (`CRM_MAIL_MODE`) | messages to contacts | address, name, message | | | | |
| Write-once storage for the audit chain head | tamper evidence | none: a hash | | | | |
| Log storage | service and access logs | what the logs hold (see the runbook) | | | | |

Services the customer runs itself (its identity provider, its SIEM, its person and organization services) are not the
supplier's sub-processors, and are listed in the customer's own record.
