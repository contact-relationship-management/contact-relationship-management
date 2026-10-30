// The one money formatter and parser (CRM-T167, CRM-T168; spec/worldwide.md, CRM-R95, CRM-D19).
//
// Amounts are integer minor units plus an ISO 4217 code, exactly as the API sends and takes them.
// The exponent (0 for JPY, 2 for EUR, 3 for KWD, 4 for CLF) and the number format come from `Intl`
// in the reader's locale, never from a hand-written `/100` or `* 100`. There is no default
// currency: a caller always passes the one the record carries or the one the person chose.
//
// THREE APPS SHARE THIS BEHAVIOUR and should change together: this file, the project portfolio
// management UI's `src/lib/money.ts` (its commit 956d9ed) and the workforce planning management
// UI's `src/lib/money.ts` (WPM-T373, WPM-T376). The exponent, the formatting and the parsing rules
// are the same in all three; copy a fix to the other two (there is no shared package, by the family
// convention of copy-adapting between front-ends). The differences here are the CRM's conventions:
// an unknown amount or a missing currency renders as an em dash ("—"), as in WPM; there is no
// `moneyWhole` (WPM's own); and `minorToMajor` / `majorToMinor` are the CRM's, for the grid and
// board columns that sort or edit a plain number.
//
// One deliberate difference from the two siblings: the exponent comes from the ISO 4217 table
// below, the same table as the service's `src/rules/currency.rs` (a unit test holds the two
// equal), and only falls back to `Intl` for a code the table does not hold. `Intl` takes its
// digits from CLDR, which differs from ISO 4217 for 16 codes (AFN, ALL, COP, HUF, IDR, IQD, IRR,
// KPW, LAK, LBP, MGA, MMK, PKR, SOS, SYP, YER: CLDR shows 0 places, ISO 4217 has 2, or 3 for IQD).
// The service converts by the ISO exponent, so a UI that typed and showed amounts by CLDR's would
// store a forint deal at a hundredth of what the service converts. The siblings still use
// `Intl`'s digits; that is recorded in spec/worldwide.md for the three apps to settle together.

import { PageError } from "./errors.js";
import { i18n, t } from "./i18n.svelte.js";

/** The reader's locale: the one in the URL. */
export function currentLocale(): string {
  return i18n.locale;
}

/**
 * ISO 4217 minor-unit exponents: the active codes, as in the service's `src/rules/currency.rs`
 * (a unit test compares the two). Funds and metals with no minor unit are left out.
 */
// prettier-ignore
export const MINOR_UNITS: Readonly<Record<string, number>> = {
  AED: 2, AFN: 2, ALL: 2, AMD: 2, ANG: 2, AOA: 2, ARS: 2, AUD: 2,
  AWG: 2, AZN: 2, BAM: 2, BBD: 2, BDT: 2, BGN: 2, BHD: 3, BIF: 0,
  BMD: 2, BND: 2, BOB: 2, BRL: 2, BSD: 2, BTN: 2, BWP: 2, BYN: 2,
  BZD: 2, CAD: 2, CDF: 2, CHF: 2, CLF: 4, CLP: 0, CNY: 2, COP: 2,
  CRC: 2, CUP: 2, CVE: 2, CZK: 2, DJF: 0, DKK: 2, DOP: 2, DZD: 2,
  EGP: 2, ERN: 2, ETB: 2, EUR: 2, FJD: 2, FKP: 2, GBP: 2, GEL: 2,
  GHS: 2, GIP: 2, GMD: 2, GNF: 0, GTQ: 2, GYD: 2, HKD: 2, HNL: 2,
  HTG: 2, HUF: 2, IDR: 2, ILS: 2, INR: 2, IQD: 3, IRR: 2, ISK: 0,
  JMD: 2, JOD: 3, JPY: 0, KES: 2, KGS: 2, KHR: 2, KMF: 0, KPW: 2,
  KRW: 0, KWD: 3, KYD: 2, KZT: 2, LAK: 2, LBP: 2, LKR: 2, LRD: 2,
  LSL: 2, LYD: 3, MAD: 2, MDL: 2, MGA: 2, MKD: 2, MMK: 2, MNT: 2,
  MOP: 2, MRU: 2, MUR: 2, MVR: 2, MWK: 2, MXN: 2, MYR: 2, MZN: 2,
  NAD: 2, NGN: 2, NIO: 2, NOK: 2, NPR: 2, NZD: 2, OMR: 3, PAB: 2,
  PEN: 2, PGK: 2, PHP: 2, PKR: 2, PLN: 2, PYG: 0, QAR: 2, RON: 2,
  RSD: 2, RUB: 2, RWF: 0, SAR: 2, SBD: 2, SCR: 2, SDG: 2, SEK: 2,
  SGD: 2, SHP: 2, SLE: 2, SOS: 2, SRD: 2, SSP: 2, STN: 2, SVC: 2,
  SYP: 2, SZL: 2, THB: 2, TJS: 2, TMT: 2, TND: 3, TOP: 2, TRY: 2,
  TTD: 2, TWD: 2, TZS: 2, UAH: 2, UGX: 0, USD: 2, UYI: 0, UYU: 2,
  UYW: 4, UZS: 2, VED: 2, VES: 2, VND: 0, VUV: 0, WST: 2, XAF: 0,
  XCD: 2, XCG: 2, XOF: 0, XPF: 0, YER: 2, ZAR: 2, ZMW: 2, ZWG: 2,
};

/**
 * Minor-unit exponent of `currency`: the ISO 4217 table's; for a code it does not hold, what
 * `Intl` says in `locale`; 2 when neither knows the code.
 */
export function exponentOf(
  currency: string,
  locale: string = currentLocale(),
): number {
  const iso = MINOR_UNITS[currency];
  if (iso !== undefined) return iso;
  try {
    return (
      new Intl.NumberFormat(locale, {
        style: "currency",
        currency,
      }).resolvedOptions().maximumFractionDigits ?? 2
    );
  } catch {
    return 2;
  }
}

function format(
  minor: number,
  currency: string,
  locale: string,
  options: Intl.NumberFormatOptions = {},
): string {
  const e = exponentOf(currency, locale);
  try {
    return new Intl.NumberFormat(locale, {
      style: "currency",
      currency,
      minimumFractionDigits: e,
      maximumFractionDigits: e,
      ...options,
    }).format(minor / 10 ** e);
  } catch {
    return `${currency} ${minor}`;
  }
}

/**
 * `minor` units of `currency` in the reader's own number format. Always names the currency.
 * `null`, `undefined` or no currency renders as an em dash (the masked or absent state), never 0.
 */
export function money(
  minor: number | null | undefined,
  currency: string | null | undefined,
  locale: string = currentLocale(),
): string {
  if (minor === null || minor === undefined || !currency) return "—";
  return format(minor, currency, locale);
}

/** Money with the sign always shown ("+1,200.00", "-5"), for a change. */
export function signedMoney(
  minor: number,
  currency: string,
  locale: string = currentLocale(),
): string {
  const text = money(Math.abs(minor), currency, locale);
  return minor > 0 ? `+${text}` : minor < 0 ? `-${text}` : text;
}

/** Plain amount for a text input ("1234,50" in a decimal-comma locale), no symbol, no grouping. */
export function minorToInput(
  minor: number,
  currency: string,
  locale: string = currentLocale(),
): string {
  const e = exponentOf(currency, locale);
  try {
    return new Intl.NumberFormat(locale, {
      minimumFractionDigits: e,
      maximumFractionDigits: e,
      useGrouping: false,
    }).format(minor / 10 ** e);
  } catch {
    return String(minor / 10 ** e);
  }
}

function symbols(locale: string): {
  decimal: string;
  group: string;
  digits: string[];
} {
  let decimal = ".";
  let group = ",";
  const digits: string[] = [];
  try {
    const nf = new Intl.NumberFormat(locale);
    for (const p of nf.formatToParts(1234567.5)) {
      if (p.type === "decimal") decimal = p.value;
      if (p.type === "group") group = p.value;
    }
    for (let d = 0; d < 10; d++) digits.push(nf.format(d));
  } catch {
    for (let d = 0; d < 10; d++) digits.push(String(d));
  }
  return { decimal, group, digits };
}

/**
 * An amount typed in `locale`'s own format to a number of major units, or `null` when it is not a
 * plain number. The locale's decimal mark is the only decimal mark ("1,5" and "1 234,50" in a
 * decimal-comma locale, "1,234.50" in an English one); its group mark and any space are ignored, and
 * the locale's own digits (০–৯, ٠–٩) are read as digits.
 */
export function parseAmount(
  text: string,
  locale: string = currentLocale(),
): number | null {
  const { decimal, group, digits } = symbols(locale);
  let s = text.trim();
  if (s === "") return null;
  s = s.replace(/[\s\u00a0\u202f']/g, "");
  if (group !== decimal && group.trim() !== "") s = s.split(group).join("");
  s = s.split(decimal).join(".");
  s = [...s]
    .map((c) => {
      const i = digits.indexOf(c);
      return i >= 0 ? String(i) : c;
    })
    .join("");
  if (!/^-?(\d+\.?\d*|\.\d+)$/.test(s)) return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

/** An amount typed in `locale` to minor units of `currency`, or `null` when it is not a number. */
export function toMinor(
  text: string,
  currency: string,
  locale: string = currentLocale(),
): number | null {
  const n = parseAmount(text, locale);
  if (n === null) return null;
  const e = exponentOf(currency, locale);
  // "1.005e2" is exactly 100.5, where 1.005 * 100 is 100.49999…
  const scaled = Number(`${n}e${e}`);
  return Number.isFinite(scaled) ? Math.round(scaled) : null;
}

/** A currency code as typed: three letters, upper-cased; `null` when it is not three letters. */
export function currencyCode(text: string): string | null {
  const c = text.trim();
  return /^[A-Za-z]{3}$/.test(c) ? c.toUpperCase() : null;
}

/** `minor` units of `currency` as a plain number of major units (`1500` yen is 1500, `150050` euro cents 1500.5). */
export function minorToMajor(
  minor: number,
  currency: string,
  locale: string = currentLocale(),
): number {
  // "150050e-2" is exactly 1500.5, with no float error from a division.
  return Number(`${minor}e-${exponentOf(currency, locale)}`);
}

/** A plain number of major units of `currency` as minor units, or `null` when it is not finite. */
export function majorToMinor(
  major: number,
  currency: string,
  locale: string = currentLocale(),
): number | null {
  if (!Number.isFinite(major)) return null;
  const scaled = Number(`${major}e${exponentOf(currency, locale)}`);
  return Number.isFinite(scaled) ? Math.round(scaled) : null;
}

/**
 * For a form: `text` typed in `locale` as minor units of the currency typed in `code`. Throws a
 * translated {@link PageError} when no currency is given (none is ever assumed) or the amount is
 * not a number in the page's own format, so the form shows why instead of sending a guess.
 */
export function formMinor(
  text: string,
  code: string | null | undefined,
  locale: string = currentLocale(),
): { minor: number; currency: string } {
  const currency = currencyCode(code ?? "");
  if (currency === null) throw new PageError(t("money.currencyRequired"));
  const minor = toMinor(text, currency, locale);
  if (minor === null) throw new PageError(t("money.amountInvalid"));
  return { minor, currency };
}
