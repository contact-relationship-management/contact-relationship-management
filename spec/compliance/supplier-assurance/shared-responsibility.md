# Shared responsibility

*Written 2026-11-20 (CRM-T276, CRM-R99).* Three parties: **the software** (what the CRM does as shipped), **the hosting
supplier** (who runs and supports it for the customer) and **the customer** (the controller of the data). "Not built"
means the software does not do it yet, so the supplier must cover it some other way or say it is missing.

Each row names its requirement and its row in the [control mapping](../control-mapping.md), where the status and the
evidence are kept. `scripts/check-assurance-pack.py` (run by `scripts/ci-check.sh lint`) fails if a row names a
requirement or a control-mapping row that does not exist, or if a requirement from CRM-R100 to CRM-R109 has no row.

| Area | Requirement | Control mapping row | The software | The hosting supplier | The customer |
| --- | --- | --- | --- | --- | --- |
| Processor duties | CRM-R99 | Processor agreements, ESP | Describes its processing in this pack, the DPIA and the record of processing | Signs the processor agreement; keeps the sub-processor register; gives breach notice within the contract's hours | Controller: completes and signs off its own DPIA |
| Records and impact assessment | CRM-R99 | Records of processing, DPIA | The DPIA and record started as far as the software goes | Adds the hosting and support facts | Completes them; its DPO and senior owner sign off |
| Sign-in and MFA | CRM-R100 | Identity provider, MFA | OIDC through Keycloak; refuses to start in production without authentication. Not built: SAML, an MFA check, turning magic links off | Connects the customer's identity provider; holds no account of its own | Runs the identity provider, MFA, joiners and leavers; assigns roles |
| Least privilege by field | CRM-R101 | Data masking by role | Masks deal money, forecasts, campaign money and contact channels by policy. Not built: the full matrix over every route, a support persona | Keeps the customer's policy as configured | Decides which roles see which fields |
| Encryption in transit | CRM-R102 | Transport encryption | Sets HSTS; refuses a database connection that could go in the clear. Not built: the same for Redis, a TLS minimum on outgoing calls | Terminates TLS 1.2 or 1.3 at the ingress; encrypts the internal network | Reviews the evidence |
| Encryption at rest | CRM-R102 | Encryption at rest | Seals two columns with AES-256-GCM, with key rotation. Not built: the other sensitive columns, attachments, a KMS | Encrypts volumes and backups; holds or brokers the keys | Decides whether it holds the keys |
| Support access | CRM-R103 | Emergency access procedure | Break-glass: reasoned, time-limited, audited, reviewed by another person. Not built: approval before use, a support persona | Uses only approved, time-limited access; vets and locates its staff as the contract asks | Approves each grant; reviews what was done |
| Audit of access | CRM-R104 | Audit of personal-data reads | Audits changes and the reads of contacts, leads and tickets. Not built: reads of deals, accounts and reports; streaming to a SIEM | Protects and keeps the logs; anchors the chain head off the host | Receives and watches the trail |
| Tamper evidence | CRM-R104 | Tamper-evident, append-only audit trail | Hash-chained, append-only trail with a verify route | Ships the chain head to write-once storage | Verifies it when it investigates |
| Tenant isolation | CRM-R105 | Tenant isolation | Runs one deployment per database; has no shared mode; refuses a start against another deployment's database, and production refuses to start without `CONTACT_RELATIONSHIP_MANAGEMENT_DEPLOYMENT_ID` | Gives each customer its own database, store, Redis and keys | Confirms it in its assessment |
| Residency | CRM-R106 | Data residency | Makes no cross-border transfer itself; with a declared list of outside hosts, refuses to start against an undeclared service and refuses webhooks to undeclared hosts; `GET /api/operations/residency` shows the list. Not built: the database, Redis and file store are not covered | Hosts production, backups and recovery copies in the agreed region; chooses in-region sub-processors | Names the region |
| Patching | CRM-R107 | Known-vulnerability scanning | Scans dependencies on every change and each night; images pinned by digest | Deploys fixes within the agreed time; scans the running images | Agrees maintenance windows |
| Independent test | CRM-R108 | Independent penetration test | A test brief exists. No test has been done | Commissions an accredited test each year; fixes critical and high findings | Reads the report summary |
| Incidents | CRM-R99 | Breach notification | The runbook's incident steps | Watches the platform, contains an incident, notifies the customer within the contract's hours | Notifies the regulator and the people affected |
| Retention | CRM-R109 | Storage limitation | One retention horizon and an operator-run sweep. Not built: horizons per class | Runs the sweep on schedule | Sets the schedule from its records policy |
| Exit | CRM-R109 | Exit export and certified erasure | Subject-access export per contact; table exports in the browser. Not built: a whole-deployment export, erasure by key destruction | Exports everything at the end of the contract, destroys its copies and certifies it | Checks the export before erasure |

## Not done

The rows describe the software today. A row whose software cell says "Not built" is a gap the supplier must cover or
declare. The supplier's and the customer's cells are the usual division, not a contract.
