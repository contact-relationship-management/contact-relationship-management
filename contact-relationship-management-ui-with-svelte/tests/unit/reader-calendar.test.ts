// The reader's calendar (CRM-T172; spec/worldwide.md code rows 29, 32 and 33): today in the reader's
// time zone, and the week's first day and weekend from CLDR for the reader's region, with a fallback
// table that agrees with the runtime's CLDR data.
//
// Ported from the workforce planning management UI's `tests/unit/reader-calendar.test.ts`
// (WPM-T378, WPM-D149), whose module this one shares in behaviour; the cases are WPM's, plus the
// CRM's own locales (`de-de`, `zh-cn`, `cy-001`) and the follow-ups page's old fixed weekend.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import {
  calendarLocale,
  FALLBACK_REGIONS,
  isoDateIn,
  readerRegion,
  todayIso,
  weekInfo,
} from "../../src/lib/reader-calendar";

describe("today is the reader's date, not the UTC date", () => {
  it("is a day ahead of UTC in Kiritimati (UTC+14) and a day behind in Pago Pago (UTC-11)", () => {
    const noonUtc = new Date("2026-10-14T12:00:00Z");
    expect(todayIso("UTC", noonUtc)).toBe("2026-10-14");
    expect(todayIso("Pacific/Kiritimati", noonUtc)).toBe("2026-10-15");
    const earlyUtc = new Date("2026-10-14T05:00:00Z");
    expect(todayIso("Pacific/Pago_Pago", earlyUtc)).toBe("2026-10-13");
    // Near midnight in India, the UTC date is still yesterday.
    expect(todayIso("Asia/Kolkata", new Date("2026-12-31T19:00:00Z"))).toBe(
      "2027-01-01",
    );
  });

  it("places an instant on its day in a zone, in ASCII digits and the Gregorian calendar", () => {
    const at = new Date("2026-10-30T22:30:00Z");
    expect(isoDateIn(at, "Europe/London")).toBe("2026-10-30");
    expect(isoDateIn(at, "Asia/Tokyo")).toBe("2026-10-31");
    expect(isoDateIn(at, "America/Los_Angeles")).toBe("2026-10-30");
    expect(isoDateIn(at, "Not/AZone")).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it("uses the runtime's zone when none is given", () => {
    expect(todayIso()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});

describe("the reader's region", () => {
  it("is the page locale's country, else the browser's first country, else none", () => {
    expect(readerRegion("en-us", ["en-GB"])).toBe("US");
    expect(readerRegion("ar-001", ["ar-EG", "en-US"])).toBe("EG");
    expect(readerRegion("ar-001", ["ar", "fr-FR"])).toBe("FR");
    expect(readerRegion("ar-001", ["ar", "en"])).toBeNull();
    expect(readerRegion("ar-001")).toBeNull();
  });
});

describe("week start and weekend from CLDR", () => {
  it("a world locale is Monday-first with a Saturday–Sunday weekend (CLDR's 001), Arabic included", () => {
    for (const loc of [
      "en-001",
      "ar-001",
      "cy-001",
      "es-001",
      "fr-001",
      "hi-001",
    ]) {
      const w = weekInfo(loc);
      expect([w.firstDay, w.weekend], loc).toEqual([1, [6, 0]]);
      expect(w.region).toBeNull();
    }
  });

  it("a Saturday week start and a Friday–Saturday weekend for a reader in Egypt on ar-001", () => {
    const w = weekInfo("ar-001", ["ar-EG"]);
    expect(w).toMatchObject({ firstDay: 6, weekend: [5, 6], region: "EG" });
    expect(calendarLocale("ar-001", w)).toBe("ar-001-u-fw-sat");
  });

  it("a Sunday week start and a Friday–Saturday weekend in Saudi Arabia; Sunday-first in the US", () => {
    expect(weekInfo("ar-001", ["ar-SA"])).toMatchObject({
      firstDay: 0,
      weekend: [5, 6],
    });
    expect(weekInfo("en-us")).toMatchObject({ firstDay: 0, weekend: [6, 0] });
    expect(calendarLocale("en-us", weekInfo("en-us"))).toBe("en-US-u-fw-sun");
  });

  it("a Thursday–Friday weekend in Afghanistan, Friday only in Iran, Sunday only in India", () => {
    expect(weekInfo("ur-001", ["ps-AF"]).weekend).toEqual([4, 5]);
    expect(weekInfo("en-001", ["fa-IR"]).weekend).toEqual([5]);
    expect(weekInfo("hi-001", ["hi-IN"])).toMatchObject({
      firstDay: 0,
      weekend: [0],
    });
  });

  it("a regional CRM locale follows its own country", () => {
    expect(weekInfo("de-de")).toMatchObject({
      firstDay: 1,
      weekend: [6, 0],
      region: "DE",
    });
    expect(weekInfo("zh-cn")).toMatchObject({
      firstDay: 1,
      weekend: [6, 0],
      region: "CN",
    });
    expect(calendarLocale("de-de", weekInfo("de-de"))).toBe("de-DE-u-fw-mon");
  });

  it("the fallback table gives the same week as the runtime's CLDR data for every region it names, and for others", () => {
    const differ: string[] = [];
    for (const region of [
      ...FALLBACK_REGIONS,
      "GB",
      "DE",
      "FR",
      "AE",
      "TR",
      "NG",
      "BR",
      "CN",
    ]) {
      const tag = `en-${region.toLowerCase()}`;
      const intl = weekInfo(tag);
      const table = weekInfo(tag, [], { intl: false });
      expect(intl.source).toBe("intl");
      expect(table.source).toBe("table");
      if (
        intl.firstDay !== table.firstDay ||
        intl.weekend.join() !== table.weekend.join()
      )
        differ.push(
          `${region}: intl ${intl.firstDay}/${intl.weekend} table ${table.firstDay}/${table.weekend}`,
        );
    }
    expect(differ, differ.join("\n")).toEqual([]);
    expect(weekInfo("ar-001", [], { intl: false })).toMatchObject({
      firstDay: 1,
      weekend: [6, 0],
    });
  });
});

describe("no page takes today as the UTC date, or a fixed weekend", () => {
  it("finds no `new Date().toISOString().slice(0, 10)` or `weekendDays={[…]}` in src/", () => {
    const here = join(__dirname, "..", "..");
    const walk = (dir: string): string[] =>
      readdirSync(dir).flatMap((name) => {
        const path = join(dir, name);
        return statSync(path).isDirectory()
          ? walk(path)
          : /\.(svelte|ts)$/.test(path)
            ? [path]
            : [];
      });
    const offenders = walk(join(here, "src"))
      // The tour's stand-in for the service answers as the service does: its "today" is the
      // UTC date until CRM-T171 gives the API an organization time zone.
      .filter((f) => !/server[/\\]demo\.ts$/.test(f))
      .filter((f) =>
        /new Date\(\)\.toISOString\(\)\.slice\(0, 10\)|weekendDays=\{\[/.test(
          readFileSync(f, "utf8"),
        ),
      )
      .map((f) => relative(here, f));
    expect(offenders).toEqual([]);
  });
});
