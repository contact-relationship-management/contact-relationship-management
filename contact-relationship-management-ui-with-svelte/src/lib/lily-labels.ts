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
import bn from "@lilydesignsystem/svelte-locale-translations/bn-001";
import cy from "@lilydesignsystem/svelte-locale-translations/cy-001";
import da from "@lilydesignsystem/svelte-locale-translations/da-001";
import de from "@lilydesignsystem/svelte-locale-translations/de-001";
import en from "@lilydesignsystem/svelte-locale-translations/en-001";
import es from "@lilydesignsystem/svelte-locale-translations/es-001";
import fr from "@lilydesignsystem/svelte-locale-translations/fr-001";
import hi from "@lilydesignsystem/svelte-locale-translations/hi-001";
import id from "@lilydesignsystem/svelte-locale-translations/id-001";
import it from "@lilydesignsystem/svelte-locale-translations/it-001";
import ja from "@lilydesignsystem/svelte-locale-translations/ja-001";
import ko from "@lilydesignsystem/svelte-locale-translations/ko-001";
import nl from "@lilydesignsystem/svelte-locale-translations/nl-001";
import pt from "@lilydesignsystem/svelte-locale-translations/pt-001";
import ru from "@lilydesignsystem/svelte-locale-translations/ru-001";
import sv from "@lilydesignsystem/svelte-locale-translations/sv-001";
import sw from "@lilydesignsystem/svelte-locale-translations/sw-001";
import ur from "@lilydesignsystem/svelte-locale-translations/ur-001";
import vi from "@lilydesignsystem/svelte-locale-translations/vi-001";
import zh from "@lilydesignsystem/svelte-locale-translations/zh-001";

/** The app's locales mapped to the packs that cover them (`de-de` is `de-001`, `zh-cn` is `zh-001`). */
const PACKS: Record<string, LabelPack> = {
  "ar-001": ar as LabelPack,
  "bn-001": bn as LabelPack,
  "cy-001": cy as LabelPack,
  "da-001": da as LabelPack,
  "de-de": de as LabelPack,
  "en-001": en as LabelPack,
  "es-001": es as LabelPack,
  "fr-001": fr as LabelPack,
  "hi-001": hi as LabelPack,
  "id-001": id as LabelPack,
  "it-001": it as LabelPack,
  "ja-001": ja as LabelPack,
  "ko-001": ko as LabelPack,
  "nl-001": nl as LabelPack,
  "pt-001": pt as LabelPack,
  "ru-001": ru as LabelPack,
  "sv-001": sv as LabelPack,
  "sw-001": sw as LabelPack,
  "ur-001": ur as LabelPack,
  "vi-001": vi as LabelPack,
  "zh-cn": zh as LabelPack,
};

/** The ready `labels` for a component in a locale; English for an unknown one. */
export function packLabels<T extends object = Record<string, unknown>>(
  helper: LabelHelper,
  locale: string,
): T {
  return labelsFor(helper, PACKS[locale] ?? (en as LabelPack)) as T;
}
