// CRM-R97: the footer links to the privacy notice on the project's site, in the UI's language where the site has it.
import { existsSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { describe, expect, it } from "vitest";
import { LOCALES } from "../../src/lib/i18n.svelte";
import { SITE_URL, privacyNoticeUrl } from "../../src/lib/privacy";

const notice = (locale: string) =>
  resolvePath(__dirname, `../../../locales/${locale}/privacy/index.md`);

describe("the privacy notice link", () => {
  it("is the notice in the UI's language where the site has one, and the English one otherwise", () => {
    expect(privacyNoticeUrl("cy-001")).toBe(`${SITE_URL}/cy-001/privacy/`);
    expect(privacyNoticeUrl("ar-001")).toBe(`${SITE_URL}/ar-001/privacy/`);
    expect(privacyNoticeUrl("de-de")).toBe(`${SITE_URL}/privacy/`);
  });

  it("names a notice that exists for every UI locale", () => {
    for (const locale of LOCALES) {
      const url = privacyNoticeUrl(locale);
      if (url === `${SITE_URL}/privacy/`) {
        expect(existsSync(notice(locale)), locale).toBe(false);
      } else {
        expect(url).toBe(`${SITE_URL}/${locale}/privacy/`);
        expect(existsSync(notice(locale)), locale).toBe(true);
      }
    }
  });
});
