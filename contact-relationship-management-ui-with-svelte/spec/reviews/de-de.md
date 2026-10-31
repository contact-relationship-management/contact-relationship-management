# Review record: `de-de`

German (Germany). One record per locale (CRM-T175); the counts are in the [register](../translation-register.md),
which a test keeps true.

| Item | Value |
| --- | --- |
| Status | draft: machine-written by Claude (the AI assistant), **not reviewed** by a native speaker |
| Interface strings | `src/lib/locales/de-de.ts` |
| Public consent pages, the confirmation mail and the form's answers | the service's `src/rules/public_text.rs`, language `de` |
| Template fallbacks | the service's `src/rules/templates.rs`, `FALLBACKS`, `de` |
| Import header names | the service's `src/rules/column_names.rs`, `de` |
| Reviewer | none |
| Reviewed on | never |

## What a review covers

1. Every interface string, read in context in the running tour, not only in the file, with the
   strings identical to English in [locale-review.md](../locale-review.md).
2. Each plural group's forms for this language (`tests/unit/plurals.test.ts` holds which forms exist).
3. The words a data subject reads: the confirmation, preference and unsubscribe pages, the web form's confirmation mail, its answer (CRM-T180) and its refusals (CRM-T182), and the
   template fallbacks, in the service.
4. The import header names a spreadsheet in this language would use.

## Known weak spots

- **Regional code, kept as a recorded limit (CRM-T175).** The locale is `de-de`, not a world `de` locale, so an Austrian, Swiss or Liechtenstein reader is served German labelled for Germany. Renaming it is not cheap or safe here: the code is the address (`/de-de/…`), the stored choice in readers' browsers, the Lily locale packs' key, the site's paths and many tests; and Swiss German conventions (`ss` for `ß`, number grouping with an apostrophe) would need a separate `de-ch` anyway. Numbers and dates follow the reader's locale through `Intl`, not this code.
- Formal address (`Sie`) throughout.

## Review log

No review yet. A reviewer adds a row here, and the register's row is changed in the same commit.

| Date | Reviewer | Scope | Outcome |
| --- | --- | --- | --- |
