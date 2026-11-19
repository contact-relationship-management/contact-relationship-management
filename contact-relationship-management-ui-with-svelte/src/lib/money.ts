// The one money formatter and parser (CRM-T167, CRM-T168; spec/worldwide.md, CRM-R95, CRM-D19).
//
// Adapted from the project portfolio and workforce planning UIs. The CRM uses the service's ISO
// exponents even where CLDR displays fewer places, e.g. IQD. Intl supplies the reader's digits,
// separators and grouping. A caller supplies the record's currency or the one the reader selected.
// The parser also supplies plain decimal text for FX rates (CRM-T268).

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

/** Plain ASCII decimal text for the API, preserving fractional digits entered by the reader. */
export function parseDecimalInput(
  text: string,
  locale: string = currentLocale(),
): string | null {
  let value = text.trim();
  if (!value) return null;
  let nf: Intl.NumberFormat;
  try {
    nf = new Intl.NumberFormat(locale);
  } catch {
    nf = new Intl.NumberFormat("en");
  }
  // A long sample exposes both group widths (e.g. 12,34,56,789) and any mark preceding the sign.
  const parts = nf.formatToParts(-123456789.5);
  const decimal = parts.find((part) => part.type === "decimal")!.value;
  const group = parts.find((part) => part.type === "group")?.value;
  const firstInteger = parts.findIndex((part) => part.type === "integer");
  const minus = parts
    .slice(0, firstInteger)
    .map((part) => part.value)
    .join("");
  const sign = [minus, "-"].find(
    (prefix) => prefix && value.startsWith(prefix),
  );
  if (sign) value = value.slice(sign.length);

  const digits = Array.from({ length: 10 }, (_, d) => nf.format(d));
  value = Array.from(value, (c) => {
    const digit = digits.indexOf(c);
    return digit < 0 ? c : String(digit);
  }).join("");
  const [integer = "", fraction = "", ...extra] = value.split(decimal);
  if (extra.length || !/^\d*$/.test(fraction)) return null;

  let whole = integer;
  if (group) {
    const grouped =
      group.trim() === "" ? whole.replace(/[ \u00a0\u202f]/g, group) : whole;
    const groups = grouped.split(group);
    if (groups.length > 1) {
      const widths = parts
        .filter((part) => part.type === "integer")
        .map((part) => Array.from(part.value).length);
      const lastWidth = widths.at(-1)!;
      const repeatWidth = widths.at(-2)!;
      if (
        !groups[0] ||
        groups[0].length > repeatWidth ||
        groups.at(-1)!.length !== lastWidth ||
        groups.slice(1, -1).some((part) => part.length !== repeatWidth)
      )
        return null;
      whole = groups.join("");
    }
  }
  if (!/^\d*$/.test(whole) || (!whole && !fraction)) return null;
  return (sign ? "-" : "") + (whole || "0") + (fraction ? "." + fraction : "");
}

/** A finite number typed in the page's own format. */
export function parseAmount(
  text: string,
  locale: string = currentLocale(),
): number | null {
  const decimal = parseDecimalInput(text, locale);
  if (decimal === null) return null;
  const n = Number(decimal);
  return Number.isFinite(n) ? n : null;
}

/** An amount typed in `locale` to minor units of `currency`, or `null` when it is not a number. */
export function toMinor(
  text: string,
  currency: string,
  locale: string = currentLocale(),
): number | null {
  const n = parseDecimalInput(text, locale);
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
