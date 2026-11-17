// The privacy notice (docs/privacy-notice.md, CRM-R97) is published on the project's site, in the site's locales.
// The footer links to it in the UI's language when the site has that language, and in English otherwise.

export const SITE_URL = "https://contact-relationship-management.github.io";

/** The site's locales with the notice (`locales/<locale>/privacy/`) that a UI locale maps to directly. */
const NOTICE_LOCALES = [
  "ar-001",
  "cy-001",
  "en-001",
  "hi-001",
  "zh-cn",
] as const;

/** The address of the privacy notice for the UI locale `locale`. */
export function privacyNoticeUrl(locale: string): string {
  return (NOTICE_LOCALES as readonly string[]).includes(locale)
    ? `${SITE_URL}/${locale}/privacy/`
    : `${SITE_URL}/privacy/`;
}
