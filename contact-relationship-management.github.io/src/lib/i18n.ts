// The site's own words (navigation, headings, footer), in the language of the page, the way the CRM UI does it
// (`contact-relationship-management-ui-with-svelte/src/lib/i18n.svelte.ts`): a table of strings per language keyed
// by a stable dotted key, English the source of truth, every table the same keys (the type holds them to it), and a
// locale resolved by its language, so `cy-gb` and `cy-001` both read Welsh and `zh-001` reads Chinese.
//
// The pages that are a role brief, or a locale's list of briefs, are in that locale; every other page is English,
// because its document is. The Arabic, Welsh, Hindi and Chinese strings are machine drafts, written by Claude and not
// reviewed by a native speaker (as the UI's are; see spec/locale/index.md).

const en = {
  skip: "Skip to content",
  site: "Site",
  "nav.home": "Home",
  "nav.install": "Install",
  "nav.bridges": "Family bridges",
  "nav.spec": "Specification",
  "nav.locales": "Locales",
  "nav.briefs": "Role briefs",
  theme: "Theme",
  textSize: "Text size",
  briefLanguage: "Language of this brief",
  allBriefs: "All role briefs",
  "briefs.lede":
    "One page for each senior role: what the software is, what it gives you, what it does not, and the decisions that are yours. Use the language button in the header for the same brief in another locale.",
  locale: "Locale:",
  source: "Source:",
  "footer.before": "Built from the documents in",
  "footer.link": "the Contact Relationship Management repository",
  "footer.after": "This site is exported from a directory of the repository itself with git subtree.",
};

/** A key of the site's strings. */
export type Key = keyof typeof en;

const ar: Record<Key, string> = {
  skip: "انتقل إلى المحتوى",
  site: "الموقع",
  "nav.home": "الرئيسية",
  "nav.install": "التثبيت",
  "nav.bridges": "جسور العائلة",
  "nav.spec": "المواصفات",
  "nav.locales": "اللغات",
  "nav.briefs": "إحاطات الأدوار",
  theme: "السمة",
  textSize: "حجم النص",
  briefLanguage: "لغة هذه الإحاطة",
  allBriefs: "كل إحاطات الأدوار",
  "briefs.lede":
    "صفحة لكل دور قيادي: ما هو البرنامج، وما يقدّمه لك، وما لا يقدّمه، والقرارات التي تعود إليك. استخدم زر اللغة في الترويسة لقراءة الإحاطة نفسها بلغة أخرى.",
  locale: "اللغة:",
  source: "المصدر:",
  "footer.before": "مبني من المستندات في",
  "footer.link": "مستودع إدارة علاقات العملاء",
  "footer.after": "يُصدَّر هذا الموقع من مجلد في المستودع نفسه باستخدام git subtree.",
};

const cy: Record<Key, string> = {
  skip: "Neidio i'r cynnwys",
  site: "Gwefan",
  "nav.home": "Hafan",
  "nav.install": "Gosod",
  "nav.bridges": "Pontydd y teulu",
  "nav.spec": "Manyleb",
  "nav.locales": "Ieithoedd",
  "nav.briefs": "Briffiau rôl",
  theme: "Thema",
  textSize: "Maint testun",
  briefLanguage: "Iaith y briff hwn",
  allBriefs: "Pob briff rôl",
  "briefs.lede":
    "Un dudalen ar gyfer pob rôl uwch: beth yw'r feddalwedd, beth mae'n ei roi i chi, beth nad yw'n ei wneud, a'r penderfyniadau sy'n eiddo i chi. Defnyddiwch y botwm iaith yn y pennawd i gael yr un briff mewn iaith arall.",
  locale: "Iaith:",
  source: "Ffynhonnell:",
  "footer.before": "Wedi'i hadeiladu o'r dogfennau yn",
  "footer.link": "ystorfa Rheoli Perthnasoedd Cwsmeriaid",
  "footer.after": "Caiff y wefan hon ei hallforio o gyfeiriadur yn yr ystorfa ei hun gyda git subtree.",
};

const hi: Record<Key, string> = {
  skip: "सामग्री पर जाएँ",
  site: "साइट",
  "nav.home": "मुखपृष्ठ",
  "nav.install": "इंस्टॉल",
  "nav.bridges": "परिवार सेतु",
  "nav.spec": "विनिर्देश",
  "nav.locales": "भाषाएँ",
  "nav.briefs": "भूमिका ब्रीफ़",
  theme: "थीम",
  textSize: "पाठ का आकार",
  briefLanguage: "इस ब्रीफ़ की भाषा",
  allBriefs: "सभी भूमिका ब्रीफ़",
  "briefs.lede":
    "हर वरिष्ठ भूमिका के लिए एक पृष्ठ: सॉफ़्टवेयर क्या है, यह आपको क्या देता है, क्या नहीं देता, और कौन-से निर्णय आपके हैं। वही ब्रीफ़ किसी दूसरी भाषा में पढ़ने के लिए शीर्ष पर भाषा बटन का उपयोग करें।",
  locale: "भाषा:",
  source: "स्रोत:",
  "footer.before": "इन दस्तावेज़ों से बनाया गया:",
  "footer.link": "संपर्क संबंध प्रबंधन रिपॉज़िटरी",
  "footer.after": "यह साइट रिपॉज़िटरी की अपनी एक निर्देशिका से git subtree द्वारा निर्यात की जाती है।",
};

const zh: Record<Key, string> = {
  skip: "跳到内容",
  site: "网站",
  "nav.home": "首页",
  "nav.install": "安装",
  "nav.bridges": "系列桥接",
  "nav.spec": "规范",
  "nav.locales": "语言区域",
  "nav.briefs": "角色简报",
  theme: "主题",
  textSize: "文字大小",
  briefLanguage: "本简报的语言",
  allBriefs: "全部角色简报",
  "briefs.lede":
    "每个高级角色一页：软件是什么、它为您提供什么、不提供什么，以及由您做出的决定。使用页眉中的语言按钮以另一种语言阅读同一份简报。",
  locale: "语言区域：",
  source: "来源：",
  "footer.before": "本站由以下仓库中的文档构建：",
  "footer.link": "客户关系管理仓库",
  "footer.after": "本站通过 git subtree 从仓库自身的一个目录导出。",
};

/** The tables, by base language. */
export const TABLES: Readonly<Record<string, Record<Key, string>>> = { en, ar, cy, hi, zh };

/** The language whose strings a locale reads: its base language when there is a table for it, else English. */
export function languageOf(locale: string | null): string {
  const base = (locale ?? "en").split("-")[0] ?? "en";
  return base in TABLES ? base : "en";
}

/** The string `key` in the language of `locale` (English on a page with no locale). */
export function t(locale: string | null, key: Key): string {
  return TABLES[languageOf(locale)]![key];
}
