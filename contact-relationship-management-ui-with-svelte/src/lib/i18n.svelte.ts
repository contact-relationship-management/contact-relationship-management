// Lightweight, dependency-free i18n for the CRM SPA (family pattern,
// copy-adapted from the WPM front-end): a per-locale strings map plus
// a reactive `$state` current-locale (Svelte 5 runes), exposed via a
// `t(key)` accessor. `en-001` is the source of truth; every locale must
// cover the same key set (the parity test pins this, and each locale file's
// type enforces it). Each locale is its own module, loaded on demand. The chosen
// locale persists to localStorage and drives the UI strings and
// `<html dir>` (RTL for `ar-001`).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`), German (`de-de`), English (`en-001`, the source of truth),
// Spanish (`es-001`), French (`fr-001`), Hindi (`hi-001`) and Simplified
// Chinese for China (`zh-cn`). `-001` is the UN M.49 code for "world": a
// language with no regional variant.

import { browser } from "$app/env";

/**
 * Locales the UI is translated into, sorted alphabetically by code (the
 * LocalePicker shows them in this order).
 */
export const LOCALES = [
  "ar-001",
  "cy-001",
  "de-de",
  "en-001",
  "es-001",
  "fr-001",
  "hi-001",
  "zh-cn",
] as const;

/** A supported locale code (one of {@link LOCALES}). */
export type Locale = (typeof LOCALES)[number];

/** Fallback locale for an unknown key, locale, or missing translation. */
export const DEFAULT_LOCALE: Locale = "en-001";

/**
 * Human-readable name for the locale switcher, written in that locale.
 * A `-001` (world) locale is labelled by its language alone; a regional
 * locale reads "<language> - <region>" (never "<language> (<region>)").
 */
export const LOCALE_LABELS: Record<Locale, string> = {
  "ar-001": "العربية",
  "cy-001": "Cymraeg",
  "de-de": "Deutsch - Deutschland",
  "en-001": "English",
  "es-001": "Español",
  "fr-001": "Français",
  "hi-001": "हिन्दी",
  "zh-cn": "中文 - 中国",
};

/** Right-to-left locales; the layout mirrors `<html dir>`. */
export const RTL_LOCALES = ["ar-001"] as const satisfies readonly Locale[];

/** Whether `locale` is written right-to-left. */
export function isRtl(locale: string): boolean {
  const resolved = normaliseLocale(locale);

  return (
    resolved !== null && (RTL_LOCALES as readonly string[]).includes(resolved)
  );
}

/** localStorage key under which the chosen UI locale persists. */
export const LOCALE_KEY = "mxi.crm.locale";

// Every translatable UI string, keyed by a stable dotted key. English is
// bundled with the app (it is the fallback and defines the keys); each other
// locale is its own module under `locales/`, fetched the first time it is
// chosen, so a visitor downloads one language, not eight.
import en from "./locales/en-001";

/** One locale's strings. */
type Table = Record<string, string>;

/** Loads a locale's module. Each `import()` is its own chunk. */
const LOADERS: Record<Locale, () => Promise<{ default: Table }>> = {
  "ar-001": () => import("./locales/ar-001"),
  "cy-001": () => import("./locales/cy-001"),
  "de-de": () => import("./locales/de-de"),
  "en-001": async () => ({ default: en }),
  "es-001": () => import("./locales/es-001"),
  "fr-001": () => import("./locales/fr-001"),
  "hi-001": () => import("./locales/hi-001"),
  "zh-cn": () => import("./locales/zh-cn"),
};

// The locales loaded so far. Reactive, so a component that called `t(...)`
// under a locale that was still loading re-renders when its table arrives.
const loaded = $state<Partial<Record<Locale, Table>>>({ "en-001": en });
const loading = new Map<Locale, Promise<void>>();

/**
 * Fetch `locale`'s strings (once). Resolves when they are available to
 * `translate`; until then `translate` answers in English.
 */
export function loadLocale(locale: Locale): Promise<void> {
  if (loaded[locale]) return Promise.resolve();
  let pending = loading.get(locale);
  if (!pending) {
    pending = LOADERS[locale]()
      .then((module) => {
        loaded[locale] = module.default;
      })
      .finally(() => loading.delete(locale));
    loading.set(locale, pending);
  }
  return pending;
}

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof typeof en;

/** Every translatable key, for the per-locale coverage test. */
export const STRING_KEYS = Object.keys(en) as StringKey[];

/** Load every locale and return their tables, for the coverage test. */
export async function loadAllLocales(): Promise<Record<Locale, Table>> {
  await Promise.all(LOCALES.map(loadLocale));
  return Object.fromEntries(
    LOCALES.map((locale) => [locale, loaded[locale] as Table]),
  ) as Record<Locale, Table>;
}

// Normalise raw input to a supported locale, or null if unsupported.
// Exact match first (hyphen/underscore-insensitive), so "zh-CN" resolves to
// `zh-cn`; otherwise the primary language subtag picks the locale whose
// code starts with it (`en`, `en-US` -> `en-001`; `zh-TW` -> `zh-cn`).
function normaliseLocale(raw: string | null | undefined): Locale | null {
  if (!raw) return null;
  const trimmed = raw.trim().replace(/_/g, "-").toLowerCase();
  const exact = (LOCALES as readonly string[]).find((l) => l === trimmed);
  if (exact) return exact as Locale;
  const primary = trimmed.split("-")[0] ?? "";
  const byPrimary = (LOCALES as readonly string[]).find(
    (l) => l.split("-")[0] === primary,
  );

  return (byPrimary as Locale | undefined) ?? null;
}

// Seed the reactive locale from localStorage (default off the browser).
function readStoredLocale(): Locale {
  if (!browser || typeof localStorage === "undefined") return DEFAULT_LOCALE;
  return normaliseLocale(localStorage.getItem(LOCALE_KEY)) ?? DEFAULT_LOCALE;
}

// Reactive current-locale state; mutating it re-renders every `t(...)`.
let current = $state<Locale>(readStoredLocale());
// A stored non-English choice is fetched at start-up; English shows until it lands.
if (browser && current !== DEFAULT_LOCALE) void loadLocale(current);

/** Reactive current-locale store with persistence. */
export const i18n = {
  /** The currently selected locale. */
  get locale(): Locale {
    return current;
  },
  /** Switch the active locale and persist the choice. */
  set(next: string): void {
    const locale = normaliseLocale(next) ?? DEFAULT_LOCALE;
    current = locale;
    void loadLocale(locale);
    if (browser && typeof localStorage !== "undefined")
      localStorage.setItem(LOCALE_KEY, locale);
  },
  /** The list of supported locales (for the switcher). */
  get locales(): readonly Locale[] {
    return LOCALES;
  },
};

/**
 * Translate `key` in `locale` with graceful fallbacks (locale → en →
 * the key itself). Pure — unit-testable without a component.
 */
export function translate(key: StringKey, locale: Locale = current): string {
  const table = loaded[locale] ?? en;
  return table[key] ?? en[key] ?? key;
}

/** Reactive translation accessor for components: `t("dash.title")`. */
export function t(key: StringKey): string {
  return translate(key, current);
}
