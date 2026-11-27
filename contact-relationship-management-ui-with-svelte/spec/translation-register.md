# Translation-review register

Who wrote each locale's interface strings and whether a native speaker has reviewed them. One row
per file in `src/lib/locales/`.

**Status** is `source` (English, the text the others follow), `draft` (written by Claude and not
reviewed by a native speaker) or `reviewed` (a named person checked every string, on a date). A
`reviewed` row must name the reviewer and the date. **No locale other than English has been
reviewed**, whatever it looks like in a browser: until a native speaker has been through it, treat
the wording as a draft, in particular for anything a data subject reads (consent, unsubscribe,
erasure) and for right-to-left layout in Arabic.

**Kept true by a test.** `tests/unit/translation-register.test.ts` fails if a locale file has no row,
a row has no file, a count is out of date, a `reviewed` row has no reviewer or date, a locale lacks a
key English has (or has one it does not), or a string uses different `{placeholders}` from the
English one. A string that fills a count is a plural group (CRM-T173): each locale holds the CLDR
forms its own language uses (Arabic and Welsh six, Spanish and French three, Chinese, Japanese, Korean, Indonesian and Vietnamese one), so the
counts differ by locale; `tests/unit/plurals.test.ts` checks each holds exactly its forms. The strings identical to English, for a reviewer to settle, are in
[locale-review.md](locale-review.md) (`node scripts/identical-to-english.mjs > spec/locale-review.md`).

| Locale | Language | Status | Method | Reviewer | Reviewed on | Strings | Identical to English |
|---|---|---|---|---|---|---|---|
| `ar-001` | Arabic (Modern Standard, right-to-left) | `draft` | Written by Claude | — | — | 928 | 1 |
| `bn-001` | Bengali | `draft` | Written by Claude | — | — | 860 | 5 |
| `cy-001` | Welsh | `draft` | Written by Claude | — | — | 928 | 10 |
| `da-001` | Danish | `draft` | Written by Claude | — | — | 860 | 23 |
| `de-de` | German (Germany) | `draft` | Written by Claude | — | — | 860 | 34 |
| `en-001` | English (international, Oxford spelling) | `source` | Written by the maintainers | — | — | 860 | 0 |
| `es-001` | Spanish (Latin America and Spain) | `draft` | Written by Claude | — | — | 877 | 13 |
| `fr-001` | French | `draft` | Written by Claude | — | — | 877 | 36 |
| `hi-001` | Hindi | `draft` | Written by Claude | — | — | 860 | 2 |
| `id-001` | Indonesian | `draft` | Written by Claude | — | — | 843 | 18 |
| `it-001` | Italian | `draft` | Written by Claude | — | — | 877 | 15 |
| `ja-001` | Japanese | `draft` | Written by Claude | — | — | 843 | 5 |
| `ko-001` | Korean | `draft` | Written by Claude | — | — | 843 | 5 |
| `nl-001` | Dutch | `draft` | Written by Claude | — | — | 860 | 40 |
| `pt-001` | Portuguese | `draft` | Written by Claude | — | — | 877 | 20 |
| `ru-001` | Russian | `draft` | Written by Claude | — | — | 894 | 4 |
| `sv-001` | Swedish | `draft` | Written by Claude | — | — | 860 | 17 |
| `sw-001` | Swahili | `draft` | Written by Claude | — | — | 860 | 6 |
| `ur-001` | Urdu (right-to-left) | `draft` | Written by Claude | — | — | 860 | 4 |
| `vi-001` | Vietnamese | `draft` | Written by Claude | — | — | 843 | 7 |
| `zh-cn` | Chinese (Simplified, China) | `draft` | Written by Claude | — | — | 843 | 1 |

## How a locale becomes `reviewed`

1. A native speaker reads every string in context (the running tour, not only the file), including
   the ones in [locale-review.md](locale-review.md).
2. Their corrections are committed.
3. The row's status, reviewer and date are filled in, in the same commit.

Adding a locale or a key is a change to this register as well: the test names what is missing.
