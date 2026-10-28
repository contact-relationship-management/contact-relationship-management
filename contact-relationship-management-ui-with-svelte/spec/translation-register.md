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
English one. The strings identical to English, for a reviewer to settle, are in
[locale-review.md](locale-review.md) (`node scripts/identical-to-english.mjs > spec/locale-review.md`).

| Locale | Language | Status | Method | Reviewer | Reviewed on | Strings | Identical to English |
|---|---|---|---|---|---|---|---|
| `ar-001` | Arabic (Modern Standard, right-to-left) | `draft` | Written by Claude | — | — | 693 | 1 |
| `cy-001` | Welsh | `draft` | Written by Claude | — | — | 693 | 10 |
| `de-de` | German (Germany) | `draft` | Written by Claude | — | — | 693 | 32 |
| `en-001` | English (international, Oxford spelling) | `source` | Written by the maintainers | — | — | 693 | 0 |
| `es-001` | Spanish (Latin America and Spain) | `draft` | Written by Claude | — | — | 693 | 10 |
| `fr-001` | French | `draft` | Written by Claude | — | — | 693 | 31 |
| `hi-001` | Hindi | `draft` | Written by Claude | — | — | 693 | 2 |
| `zh-cn` | Chinese (Simplified, China) | `draft` | Written by Claude | — | — | 693 | 1 |

## How a locale becomes `reviewed`

1. A native speaker reads every string in context (the running tour, not only the file), including
   the ones in [locale-review.md](locale-review.md).
2. Their corrections are committed.
3. The row's status, reviewer and date are filled in, in the same commit.

Adding a locale or a key is a change to this register as well: the test names what is missing.
