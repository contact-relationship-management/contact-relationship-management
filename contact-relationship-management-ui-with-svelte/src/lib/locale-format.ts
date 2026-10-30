// The one number, percentage and date formatter for everything that is not money (CRM-T172;
// spec/worldwide.md code rows 31 to 35, CRM-R95, CRM-D19). Money is `money.ts`.
//
// SHARED IN BEHAVIOUR with the workforce planning management UI's `src/lib/locale-format.ts`
// (WPM-T375), from which this file is ported: the same functions, names, rules and fallbacks, so the
// family formats alike; copy a fix to the other (there is no shared package, by the family convention
// of copy-adapting between front-ends). The project portfolio management UI has no such module yet.
// The CRM's differences: its locales are its own eight (`de-de`, `zh-cn` and six `-001`), and WPM's
// centi-FTE helper is left out (the CRM has no FTE).
//
// Every figure a person reads goes through `Intl` in the page's locale (`i18n.locale`, the one in the
// URL), so a German page shows "1.234,5", a French one "12,5 %", and a date reads "30 Oct 2026" or
// "30.10.2026" rather than "2026-10-30". The values themselves are unchanged: the API still sends and
// takes ISO dates, basis points and fractions, and this module only turns them into text (and typed
// text back into a number, through `money.ts`'s parser). An unknown value is "—", never 0.
//
// `tests/unit/locale-format.test.ts` fails on new hand-rolled formatting in `src/` (`toFixed`,
// `toLocale*`, a percent sign glued to a number, a timestamp sliced to a date).

import { i18n } from "./i18n.svelte.js";
import { parseAmount } from "./money.js";

/** What an absent or unknown figure shows. */
export const DASH = "—";

type Num = number | null | undefined;

function locale(given?: string): string {
  return given ?? i18n.locale;
}

function nf(loc: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
  try {
    return new Intl.NumberFormat(loc, options);
  } catch {
    return new Intl.NumberFormat("en", options);
  }
}

/** A number in the page's format: grouping, the locale's decimal mark and digits, at most three decimals. */
export function formatNumber(
  n: Num,
  options: Intl.NumberFormatOptions = {},
  loc?: string,
): string {
  if (n === null || n === undefined || !Number.isFinite(n)) return DASH;
  return nf(locale(loc), options).format(n);
}

/** A count or other whole number ("1,234"); a fraction is rounded. */
export function formatInteger(n: Num, loc?: string): string {
  return formatNumber(n, { maximumFractionDigits: 0 }, loc);
}

/** A number with exactly `digits` decimals ("2.50", "2,50"): the locale's own `toFixed`. */
export function formatDecimal(n: Num, digits: number, loc?: string): string {
  return formatNumber(
    n,
    { minimumFractionDigits: digits, maximumFractionDigits: digits },
    loc,
  );
}

/** A fraction as a percentage in the locale's own form: 0.25 is "25%" in English, "25 %" in German, "%25" in Turkish. */
export function formatPercent(fraction: Num, digits = 0, loc?: string): string {
  return formatNumber(
    fraction,
    {
      style: "percent",
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    },
    loc,
  );
}

/** A value that is already in percent (8.3 means 8.3%), as a percentage. */
export function formatPercentValue(
  value: Num,
  digits = 0,
  loc?: string,
): string {
  if (value === null || value === undefined || !Number.isFinite(value))
    return DASH;
  // "8.3e-2" is exactly 0.083, where 8.3 / 100 is not.
  return formatPercent(Number(`${value}e-2`), digits, loc);
}

/** Basis points (hundredths of a percent) as a percentage: 10456 is "104.56%"; `digits` defaults to 2. */
export function formatBasisPoints(bp: Num, digits = 2, loc?: string): string {
  if (bp === null || bp === undefined || !Number.isFinite(bp)) return DASH;
  return formatPercent(Number(`${Math.trunc(bp)}e-4`), digits, loc);
}

/** A number with its sign always shown ("+3", "-1.5", "0"), for a change. */
export function formatSigned(
  n: Num,
  options: Intl.NumberFormatOptions = {},
  loc?: string,
): string {
  return formatNumber(n, { signDisplay: "exceptZero", ...options }, loc);
}

/**
 * A decimal the API sends as text ("1.27", "0.0067", an exchange rate) in the page's format ("1,27"),
 * with every digit it has: `Intl` formats the string itself, so nothing is lost to a float. Text that
 * is not a plain decimal is shown as given. The CRM's own (WPM has no decimal sent as text).
 */
export function formatDecimalText(
  text: string | null | undefined,
  loc?: string,
): string {
  if (text === null || text === undefined || text === "") return DASH;
  const s = text.trim();
  if (!/^-?\d+(?:\.\d+)?$/.test(s)) return text;
  const digits = s.split(".")[1]?.replace(/0+$/, "").length ?? 0;
  return nf(locale(loc), {
    minimumFractionDigits: 0,
    maximumFractionDigits: Math.min(digits, 100),
  }).format(s as Intl.StringNumericLiteral);
}

// Dates. A civil date ("2026-10-30", no time and no zone) is the same day everywhere, so it is formatted in UTC;
// a timestamp ("2026-10-30T09:15:00Z") is an instant, so it is shown in the reader's own time zone.

const CIVIL = /^(\d{4})-(\d{2})-(\d{2})$/;
const MONTH = /^(\d{4})-(\d{2})$/;

function instant(iso: string): { date: Date; civil: boolean } | null {
  const s = iso.trim();
  const c = CIVIL.exec(s);
  if (c) {
    const date = new Date(`${s}T00:00:00Z`);
    return Number.isNaN(date.getTime()) ? null : { date, civil: true };
  }
  if (!/^\d{4}-\d{2}-\d{2}T/.test(s)) return null;
  const date = new Date(s);
  return Number.isNaN(date.getTime()) ? null : { date, civil: false };
}

function dtf(
  loc: string,
  options: Intl.DateTimeFormatOptions,
): Intl.DateTimeFormat {
  try {
    return new Intl.DateTimeFormat(loc, options);
  } catch {
    return new Intl.DateTimeFormat("en", options);
  }
}

/** Whether `value` is an ISO civil date ("2026-10-30"). */
export function isCivilDate(value: unknown): value is string {
  return (
    typeof value === "string" && CIVIL.test(value) && instant(value) !== null
  );
}

/**
 * An ISO date or timestamp as a date in the page's locale ("30 Oct 2026" in en-001, "30.10.2026" in de-001).
 * `null`, `undefined` or "" is "—"; text that is not an ISO date is shown as given.
 */
export function formatDate(
  iso: string | null | undefined,
  style: "short" | "medium" | "long" | "full" = "medium",
  loc?: string,
): string {
  if (iso === null || iso === undefined || iso === "") return DASH;
  const at = instant(iso);
  if (!at) return iso;
  return dtf(locale(loc), {
    dateStyle: style,
    ...(at.civil ? { timeZone: "UTC" } : {}),
  }).format(at.date);
}

/** An ISO timestamp as a date and time in the page's locale and the reader's time zone. */
export function formatDateTime(
  iso: string | null | undefined,
  loc?: string,
): string {
  if (iso === null || iso === undefined || iso === "") return DASH;
  const at = instant(iso);
  if (!at) return iso;
  if (at.civil) return formatDate(iso, "medium", loc);
  return dtf(locale(loc), { dateStyle: "medium", timeStyle: "short" }).format(
    at.date,
  );
}

/** An ISO timestamp's time of day in the page's locale and the reader's time zone ("09:15", "9:15 am"). */
export function formatTime(
  iso: string | null | undefined,
  loc?: string,
): string {
  if (iso === null || iso === undefined || iso === "") return DASH;
  const at = instant(iso);
  if (!at || at.civil) return iso;
  return dtf(locale(loc), { timeStyle: "short" }).format(at.date);
}

/** A month ("2026-10" or the first of it, "2026-10-01") as a month and year ("Oct 2026"). */
export function formatMonth(
  iso: string | null | undefined,
  month: "short" | "long" = "short",
  loc?: string,
): string {
  if (iso === null || iso === undefined || iso === "") return DASH;
  const s = iso.trim();
  const m = MONTH.exec(s) ?? CIVIL.exec(s);
  if (!m) return iso;
  const date = new Date(`${m[1]}-${m[2]}-01T00:00:00Z`);
  if (Number.isNaN(date.getTime())) return iso;
  return dtf(locale(loc), { month, year: "numeric", timeZone: "UTC" }).format(
    date,
  );
}

// Typed numbers. The parsing core is `money.ts`'s `parseAmount`, so a decimal typed into any field is read the same
// way an amount is: the locale's decimal mark is the only decimal mark, its group mark and spaces are ignored, and
// its own digits are digits.

/** A number typed in the page's format ("2,5" in German), or `null` when it is not a plain number. */
export function parseNumber(text: string, loc?: string): number | null {
  return parseAmount(text, locale(loc));
}

/**
 * A number typed in the page's format, scaled by 10^`exponent` and rounded to an integer: a percentage typed as
 * "2,5" with exponent 2 is 250 basis points. `null` when it is not a number.
 */
export function parseScaled(
  text: string,
  exponent: number,
  loc?: string,
): number | null {
  const n = parseNumber(text, loc);
  if (n === null) return null;
  const scaled = Number(`${n}e${exponent}`);
  return Number.isFinite(scaled) ? Math.round(scaled) : null;
}

/** A number for a text input in the page's format, no grouping ("2,5" in German), or "" when absent. */
export function numberToInput(n: Num, loc?: string): string {
  if (n === null || n === undefined || !Number.isFinite(n)) return "";
  return nf(locale(loc), {
    useGrouping: false,
    maximumFractionDigits: 20,
  }).format(n);
}
