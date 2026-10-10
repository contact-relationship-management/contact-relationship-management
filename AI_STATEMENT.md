# AI Statement

Version 1.0.0, status is active.

Canonical location is `AI_STATEMENT.md` at the repository root.

Review at least at every major release.

Abstract: This document discloses how artificial-intelligence tools are used
to develop this software. It states what the tools do and do not touch, who is
accountable, which controls bound the work, the licensing and data posture, the
rules for contributors, the prohibited uses, and the limitations that remain.
It is a self-declaration by the maintainer, and it changes in the same pull
request that changes the practice it describes.

The key words shall, should, and may are used as ISO/IEC Directives Part 2
defines them: requirement, recommendation, permission.

## 1. Scope

This document covers the use of AI tools in developing everything in this
repository. It does not cover an AI system in the product, because there is
none. AI is used to build the software, in the same sense that autocomplete,
linters, and compilers are used to build it.

## 2. Which frameworks apply here, and which do not

- The EU AI Act imposes no obligation on this project. The Act binds providers
  and deployers of AI systems (Articles 2 and 3(1)); this software is not one.
  This document is voluntary.
- Data-protection law (GDPR, PECR) is relevant to CRM software, but it concerns
  the product's handling of personal data, not the use of AI to write it. See
  [spec/regulatory.md](spec/regulatory.md).
- ISO/IEC 42001 and the NIST AI RMF are used as vocabulary, not claimed as
  conformity. No certification is claimed and no audit has occurred.

## 3. Terms

This document uses the W3C AI Content Disclosure vocabulary:

- none: entirely human-authored
- ai-assisted: human-authored; AI edited, refined, or filled in boilerplate
- ai-generated: AI-generated with human prompting and review
- autonomous: AI-generated without meaningful human oversight
- agentic tool: a tool that plans and executes multi-step work under a
  human's direction, as opposed to inline completion

## 4. Accountability

[MAINTAINERS.md](MAINTAINERS.md) lists the accountable maintainer for every
change in this repository, whatever tool produced the bytes.

## 5. Where AI is used, and at what level

The tooling is agentic AI coding assistance (currently Claude Code, by
Anthropic), operated in sessions the maintainer directs, reviews, and merges.
No percentage appears in this document: no defensible method exists for
measuring one.

| Activity                                              | Level        | Notes                                                                 |
| ----------------------------------------------------- | ------------ | --------------------------------------------------------------------- |
| Application and tooling code                          | ai-generated | written in directed sessions; reviewed and merged by the maintainer   |
| Tests                                                 | ai-generated | held to the same authority as the code; expectations cite the spec    |
| Documentation and this statement                      | ai-generated | reviewed by the maintainer                                            |
| Specification decisions, owner rulings, release calls | none         | decided by the maintainer and recorded in the spec                    |
| Review verdicts on others' contributions              | none         | decided by the maintainer                                             |

## 6. Human oversight

The maintainer directs the work and merges changes. Decisions with
consequences, such as what a specification silence means or what ships in a
release, are recorded in [spec](spec/index.md). A decision that exists only
inside a tool session is not a decision this project made.

## 7. Quality controls

- Specification authority. The [specification](spec/index.md) is the
  conformance oracle; decisions cite file and section.
- Automated tests. Unit, request, and database-backed tests in the Rust
  service; unit and Playwright tests in the Svelte front end.
- Static and supply-chain gates. Clippy (pedantic) for Rust, lockfiles for
  dependencies, and CI on GitHub Actions and Woodpecker
  (`scripts/ci-check.sh`).

## 8. Licensing and provenance of AI output

The project is licensed as stated in [LICENSE.md](LICENSE.md). An AI tool's
output does not launder anyone's copyright, the full provenance of generated
text is generally not knowable, and prompting alone is not treated as
authorship. In practice: contributions of substantially copied third-party
material are refused however they were produced; generated code is held to the
same originality expectations and review as human code; and if identifiable
third-party material is found in the tree, it is removed or licensed properly.

## 9. Data

No real customer data, personal data, or secrets exist in this project: not in
the repository, not in test fixtures, and therefore not in any prompt. All data
is synthetic. Vendor-side data handling is governed by the tool vendor's terms;
this document makes no claim on the vendor's behalf.

## 10. Rules for contributors

Contributors may use AI tools. A contribution with ai-generated content per §3
shall say so in the pull-request description: which tool, and what it did.
The contributor remains responsible for the submission in full, under the same
[CONTRIBUTING.md](CONTRIBUTING.md) bar as any other work: understood,
explained on request, tested, and honest.

## 11. Prohibited uses

In this project, AI shall not: merge anything; adjudicate or score
contributions; sign anything; decide a specification-facing question; or
weaken a test, an expectation, or a gate to make something pass.

## 12. Limitations and residual risks

- Tests prove what they test, not correctness.
- Review depth is one person's. See [GOVERNANCE.md](GOVERNANCE.md).
- Commits predating this statement carry no disclosure markers.
- Whether any generated fragment echoes unlicensed training material is not
  fully knowable with current tools.
- The legal status of copyright in AI output is unsettled in most jurisdictions.
- This is a self-declaration; no third party has audited it.

## 13. Review and change

This statement is reviewed at every major or minor release, and revised
off-cycle when the tooling changes materially, a vendor's terms change in a way
§8 or §9 relies on, a binding rule emerges, or a claim stops being true. The
change lands as a pull request, with the version and change log updated in the
same PR.

## 14. Reporting

A suspected provenance, licensing, or quality problem, including a claim in this
document that does not survive checking, is welcome as an issue. For anything
security-sensitive, use the private route in [SECURITY.md](SECURITY.md).

## Annex A. Change log

| Version | Date       | Change       |
| ------- | ---------- | ------------ |
| 1.0.0   | 2026-10-10 | First issue. |

## Annex B. Machine-readable summary

The prose above is authoritative where the two could disagree.

```yaml
ai-statement:
  version: 1.0.0
  last-updated: 2026-10-10
  vocabulary: w3c-ai-content-disclosure
  disclosure-default: ai-generated
  tools:
    - name: Claude Code
      provider: Anthropic
  processes:
    design: ai-assisted
    implementation: ai-generated
    testing: ai-generated
    documentation: ai-generated
    review: none
    adjudication: none
    release-decisions: none
  ships-ai-system: false
  autonomous-use: none
```
