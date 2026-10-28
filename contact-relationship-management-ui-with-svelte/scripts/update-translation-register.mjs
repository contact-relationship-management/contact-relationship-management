// Refreshes the counts in `spec/translation-register.md` (the strings per locale, and how many are
// identical to English) after a key is added. It changes only the two number columns: the status,
// the reviewer and the date are a person's to write. Run
// `node scripts/update-translation-register.mjs && node scripts/identical-to-english.mjs > spec/locale-review.md`.

import { readdirSync, readFileSync, writeFileSync } from "node:fs";

const dir = new URL("../src/lib/locales/", import.meta.url);
const file = new URL("../spec/translation-register.md", import.meta.url);
const codes = readdirSync(dir)
  .filter((name) => name.endsWith(".ts"))
  .map((name) => name.slice(0, -3));
const en = (await import(new URL("en-001.ts", dir))).default;
const total = Object.keys(en).length;

let text = readFileSync(file, "utf8");
for (const code of codes) {
  const table = (await import(new URL(`${code}.ts`, dir))).default;
  const same =
    code === "en-001"
      ? 0
      : Object.keys(en).filter((key) => table[key] === en[key]).length;
  const row = new RegExp(`^(\\| \`${code}\` \\|.*\\| )\\d+ \\| \\d+ \\|$`, "m");
  if (!row.test(text)) throw new Error(`no row for ${code} in the register`);
  text = text.replace(row, `$1${total} | ${same} |`);
}
writeFileSync(file, text);
console.log(`${codes.length} locales, ${total} strings each`);
