# Locale status

Status of the role briefs under `locales/<code>/for/<slug>/` (see
[locales-x-for-roles](../locales-x-for-roles/index.md) and
[locales-for-global-sharing-with-svelte](../locales-for-global-sharing-with-svelte/index.md)).
Source text: `en-gb-oxendict`, refreshed 2026-10-11.

| Locale | Briefs | Method | Reviewed by a native speaker |
|---|---|---|---|
| en-gb-oxendict | 11 | Source (Oxford -ize spelling, British otherwise) | n/a |
| en-001 | 11 | Same text as en-gb-oxendict (Oxford spelling is the international convention used here) | n/a |
| en-gb | 11 | Spelling rules applied to the source (-ise family) | n/a |
| en-us | 11 | Spelling rules applied to the source (license, program, behavior, enrollment) | n/a |
| zh-cn | 11 | Translated by Claude (Simplified Chinese), native-script slugs | **No** |
| zh-001 | 11 | Byte copy of zh-cn | **No** |
| hi-in | 11 | Translated by Claude (Hindi), native-script slugs | **No** |
| hi-001 | 11 | Byte copy of hi-in | **No** |
| cy-gb | 11 | Translated by Claude (Welsh), Welsh slugs | **No** |
| cy-001 | 11 | Byte copy of cy-gb | **No** |
| ar-001 | 11 | Translated by Claude (Modern Standard Arabic, right-to-left), native-script slugs; no regional variant | **No** |

## Rules applied

- Each topic directory has `index.md`, a `README.md` symlink to it, and a
  `.locale-peer-id`. The id is the same 32-character hexadecimal value in every
  locale for the same topic, regardless of slug.
- English locales keep English slugs; the Chinese, Hindi and Welsh locales use
  native-language slugs, and the links between role briefs point at them.
- Kept untranslated on purpose: code, commands, identifiers, file names, task
  numbers (T39 and so on), product and standard names (PostgreSQL, Redis, HIPAA,
  GDPR, NHS DTAC, WCAG), and proper nouns.
- All relative links were checked to resolve in every locale.

## Known limits

- Translations into Arabic, Chinese, Hindi and Welsh are machine-produced and
  **unreviewed**. Legal, risk and compliance wording in particular needs a
  qualified native-speaking reviewer before anyone relies on it.
- The `-001` copies are identical to their regional sibling; no regional
  adaptation was made.
- The briefs were refreshed on 2026-10-11 for T49 (Dependabot and the signed-release workflow), T53 (response schemas), T54 (RFC 9457 errors) and the current counts (51 of 55 work items; 124 unit, 26 database, 44 front-end unit and 42 Playwright tests). Counts and states are a snapshot; refresh the English source, then every locale, when they change.
- The English facts in the source (counts such as "48 of 53" tasks, test totals)
  are a snapshot and are copied as is; refresh all locales when they change.
