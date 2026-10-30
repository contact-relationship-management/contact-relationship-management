// The reader's calendar (CRM-T172; spec/worldwide.md code rows 29, 32 and 33, CRM-R95, CRM-D19):
// "today" in the reader's own time zone, and the week's first day and weekend from Unicode CLDR week
// data for the reader's region. The API is unchanged: it sends and takes ISO dates, and a civil date
// from it is the same day everywhere (it is never shifted here).
//
// SHARED IN BEHAVIOUR with the workforce planning management UI's `src/lib/reader-calendar.ts`
// (WPM-T378, WPM-D149), from which this file is ported unchanged but for this header: the same
// region order, the same `Intl` source and the same fallback table, so the family's calendars agree;
// copy a fix to the other. The project portfolio management UI has no reader-calendar module yet.
//
// Region, in order: the page locale's own country (`de-de`, `zh-cn`); else the first country among
// the browser's languages (`navigator.languages`); else the page locale itself, whose `001` (world)
// region is Monday and Saturday–Sunday in CLDR. The language alone never picks a country. Week data
// comes from `Intl.Locale#getWeekInfo()` (or the older `weekInfo` property); where the browser has
// neither, from the small CLDR table below.
//
// "Today" here is the reader's day. An organization time zone (and the API's own "today") is
// CRM-T171's, not this file's.

/** 0 = Sunday … 6 = Saturday, as `Date#getUTCDay` and the Lily calendar count. */
export type Weekday = 0 | 1 | 2 | 3 | 4 | 5 | 6;

export interface WeekInfo {
  firstDay: Weekday;
  /** The weekend days, in week order. */
  weekend: Weekday[];
  /** The ISO 3166 country the week was taken for, or `null` for the page locale's own (world) data. */
  region: string | null;
  /** Where the week came from: the browser's CLDR data, or this module's fallback table. */
  source: "intl" | "table";
}

// Gregorian, ASCII digits, whatever the page's locale: this text is an ISO date, not something a person reads.
const ISO_PARTS = "en-US-u-ca-gregory-nu-latn";

/** The reader's IANA time zone ("Asia/Kolkata"), or `undefined` when the browser does not say. */
export function readerTimeZone(): string | undefined {
  try {
    return new Intl.DateTimeFormat().resolvedOptions().timeZone || undefined;
  } catch {
    return undefined;
  }
}

/** The ISO date ("2026-10-30") an instant falls on in `timeZone` (the reader's when absent). */
export function isoDateIn(at: Date, timeZone?: string): string {
  const options: Intl.DateTimeFormatOptions = {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  };
  let parts: Intl.DateTimeFormatPart[];
  try {
    parts = new Intl.DateTimeFormat(ISO_PARTS, {
      ...options,
      timeZone,
    }).formatToParts(at);
  } catch {
    // An unknown zone name: the runtime's own zone.
    parts = new Intl.DateTimeFormat(ISO_PARTS, options).formatToParts(at);
  }
  const part = (type: string) => parts.find((p) => p.type === type)?.value;
  return `${(part("year") ?? "").padStart(4, "0")}-${part("month")}-${part("day")}`;
}

/** Today's ISO date in the reader's time zone (or `timeZone`), not the UTC date. */
export function todayIso(timeZone?: string, now: Date = new Date()): string {
  return isoDateIn(now, timeZone ?? readerTimeZone());
}

/** A two-letter country in a language tag ("US" in "en-us"), or `null` (a bare language, or `001`). */
export function countryOf(tag: string): string | null {
  try {
    const region = new Intl.Locale(tag).region;
    return region && /^[A-Z]{2}$/.test(region) ? region : null;
  } catch {
    return null;
  }
}

/** The region whose week the reader's calendar follows (see the order above), or `null` for the page locale's own. */
export function readerRegion(
  pageLocale: string,
  readerLanguages: readonly string[] = [],
): string | null {
  const own = countryOf(pageLocale);
  if (own) return own;
  for (const tag of readerLanguages) {
    const region = countryOf(tag);
    if (region) return region;
  }
  return null;
}

// The fallback, from CLDR `weekData` (supplementalData.xml): the regions whose week is not Monday-first and
// Saturday–Sunday. The browser's `getWeekInfo` is the source; this table only answers where it is missing, and may lag
// CLDR. `tests/unit/reader-calendar.test.ts` checks it agrees with the runtime's own CLDR data for every region in it.
const SUNDAY_FIRST = new Set(
  "AG AS BD BR BS BT BW BZ CA CO DM DO ET GT GU HK HN ID IL IN JM JP KE KH KR LA MH MM MO MT MX MZ NI NP PA PE PH PK PR PT PY SA SG SV TH TT TW UM US VE VI WS YE ZA ZW".split(
    " ",
  ),
);
const SATURDAY_FIRST = new Set(
  "AF BH DJ DZ EG IQ IR JO KW LY OM QA SD SY".split(" "),
);
const FRIDAY_FIRST = new Set(["MV"]);
const FRIDAY_SATURDAY = new Set(
  "BH DZ EG IL IQ JO KW LY OM QA SA SD SY YE".split(" "),
);
const THURSDAY_FRIDAY = new Set(["AF"]);
const FRIDAY_ONLY = new Set(["IR"]);
const SUNDAY_ONLY = new Set(["IN", "UG"]);

/** Every region the fallback table names (for the test that checks it against the runtime's CLDR data). */
export const FALLBACK_REGIONS: readonly string[] = [
  ...new Set([
    ...SUNDAY_FIRST,
    ...SATURDAY_FIRST,
    ...FRIDAY_FIRST,
    ...FRIDAY_SATURDAY,
    ...THURSDAY_FRIDAY,
    ...FRIDAY_ONLY,
    ...SUNDAY_ONLY,
  ]),
].sort();

function fromTable(region: string | null): Omit<WeekInfo, "source"> {
  const r = region ?? "";
  const firstDay: Weekday = SUNDAY_FIRST.has(r)
    ? 0
    : SATURDAY_FIRST.has(r)
      ? 6
      : FRIDAY_FIRST.has(r)
        ? 5
        : 1;
  const weekend: Weekday[] = FRIDAY_SATURDAY.has(r)
    ? [5, 6]
    : THURSDAY_FRIDAY.has(r)
      ? [4, 5]
      : FRIDAY_ONLY.has(r)
        ? [5]
        : SUNDAY_ONLY.has(r)
          ? [0]
          : [6, 0];
  return { firstDay, weekend, region };
}

type WeekInfoLocale = Intl.Locale & {
  getWeekInfo?: () => { firstDay: number; weekend: number[] };
  weekInfo?: { firstDay: number; weekend: number[] };
};

/** CLDR's 1 = Monday … 7 = Sunday as 0 = Sunday … 6 = Saturday. */
const weekday = (cldr: number) => (cldr % 7) as Weekday;

/**
 * The reader's week: its first day and its weekend. `readerLanguages` is `navigator.languages` in the browser; pass
 * `{ intl: false }` to read the fallback table (for tests).
 */
export function weekInfo(
  pageLocale: string,
  readerLanguages: readonly string[] = [],
  { intl = true }: { intl?: boolean } = {},
): WeekInfo {
  const region = readerRegion(pageLocale, readerLanguages);
  if (intl) {
    try {
      const language = new Intl.Locale(pageLocale).language;
      const tag = region ? `${language}-${region}` : pageLocale;
      const loc = new Intl.Locale(tag) as WeekInfoLocale;
      const info = loc.getWeekInfo?.() ?? loc.weekInfo;
      if (info && typeof info.firstDay === "number") {
        return {
          firstDay: weekday(info.firstDay),
          weekend: info.weekend.map(weekday),
          region,
          source: "intl",
        };
      }
    } catch {
      // A malformed tag: the table.
    }
  }
  return { ...fromTable(region), source: "table" };
}

const FW = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"] as const;

/**
 * The tag the Lily calendar takes its first day from: the page locale (so its formatting is the page's) with the
 * CLDR `fw` keyword set to the reader's first day ("ar-001-u-fw-sat").
 */
export function calendarLocale(pageLocale: string, week: WeekInfo): string {
  try {
    return new Intl.Locale(pageLocale, {
      firstDayOfWeek: FW[week.firstDay],
    } as Intl.LocaleOptions).toString();
  } catch {
    return `${pageLocale}-u-fw-${FW[week.firstDay]}`;
  }
}

/** The browser's languages, or none outside a browser. */
export function browserLanguages(): readonly string[] {
  try {
    return typeof navigator === "undefined" ? [] : navigator.languages;
  } catch {
    return [];
  }
}
