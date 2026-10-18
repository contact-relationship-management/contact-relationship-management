// Lily's label packs (`@lilydesignsystem/svelte-locale-translations`): every label a
// grid-like component needs, per language, so the app writes only what is its
// own (its record names, its column headings) and the generic wording (export,
// copy, search, undo, announcements) comes from the pack. The packs other than
// English are machine drafts that no native speaker has read (`pack.reviewed`).
import {
  labelsFor,
  type LabelHelper,
  type LabelPack,
} from "@lilydesignsystem/svelte-locale-translations";
import ar from "@lilydesignsystem/svelte-locale-translations/ar-001";
import cy from "@lilydesignsystem/svelte-locale-translations/cy-001";
import de from "@lilydesignsystem/svelte-locale-translations/de-001";
import en from "@lilydesignsystem/svelte-locale-translations/en-001";
import es from "@lilydesignsystem/svelte-locale-translations/es-001";
import fr from "@lilydesignsystem/svelte-locale-translations/fr-001";
import hi from "@lilydesignsystem/svelte-locale-translations/hi-001";
import zh from "@lilydesignsystem/svelte-locale-translations/zh-001";

/** The app's locales mapped to the packs that cover them (`de-de` is `de-001`). */
const PACKS: Record<string, LabelPack> = {
  "ar-001": ar as LabelPack,
  "cy-001": cy as LabelPack,
  "de-de": de as LabelPack,
  "en-001": en as LabelPack,
  "es-001": es as LabelPack,
  "fr-001": fr as LabelPack,
  "hi-001": hi as LabelPack,
  "zh-cn": zh as LabelPack,
};

/** The ready `labels` for a component in a locale; English for an unknown one. */
export function packLabels<T extends object = Record<string, unknown>>(
  helper: LabelHelper,
  locale: string,
): T {
  return labelsFor(helper, PACKS[locale] ?? (en as LabelPack)) as T;
}
