# Review record: `zh-cn`

Chinese (Simplified, China). One record per locale (CRM-T175); the counts are in the [register](../translation-register.md),
which a test keeps true.

| Item | Value |
| --- | --- |
| Status | draft: machine-written by Claude (the AI assistant), **not reviewed** by a native speaker |
| Interface strings | `src/lib/locales/zh-cn.ts` |
| Public consent pages | the service's `src/rules/public_text.rs`, language `zh` |
| Template fallbacks | the service's `src/rules/templates.rs`, `FALLBACKS`, `zh` |
| Import header names | the service's `src/rules/column_names.rs`, `zh` |
| Reviewer | none |
| Reviewed on | never |

## What a review covers

1. Every interface string, read in context in the running tour, not only in the file, with the
   strings identical to English in [locale-review.md](../locale-review.md).
2. Each plural group's forms for this language (`tests/unit/plurals.test.ts` holds which forms exist).
3. The words a data subject reads: the confirmation, preference and unsubscribe pages and the
   template fallbacks, in the service.
4. The import header names a spreadsheet in this language would use.

## Known weak spots

- **Regional code, kept as a recorded limit (CRM-T175).** The locale is `zh-cn`, Simplified Chinese labelled for mainland China; readers who use Traditional Chinese (Taiwan, Hong Kong, Macao) are served Simplified, and there is no `zh-Hant` locale. Renaming the code to `zh-Hans` would not add Traditional Chinese, and it has the same cost as for `de-de` (addresses, stored choices, the Lily packs' key, the site, tests). A Traditional Chinese locale is a new locale to add, not a rename.
- One plural form (`other`), as CLDR gives for `zh`.

## Review log

No review yet. A reviewer adds a row here, and the register's row is changed in the same commit.

| Date | Reviewer | Scope | Outcome |
| --- | --- | --- | --- |
