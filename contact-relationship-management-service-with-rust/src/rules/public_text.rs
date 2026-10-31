//! The words of the public consent pages, by language (CRM-T175, CRM-R29, CRM-R95).
//!
//! The confirmation, preference and unsubscribe pages are what a data subject reads about their
//! own consent, so they are in the reader's language: the one the reader's browser asks for
//! (`Accept-Language`, RFC 9110 §12.5.4, with its `q` weights), unless the page's address names
//! one (`?lang=`, from the language links every page carries). The contact's language is not
//! stored: the browser says it on every visit, and storing it would be one more piece of personal
//! data kept for no other use. A language not here falls back to English, which is recorded as a
//! limit. The eight languages are the UI's; every one but English is a machine draft by the AI
//! assistant, unreviewed by a native speaker (the UI's `spec/translation-register.md`).

/// One language's words. `{purpose}` in a template is replaced by the purpose's words.
#[derive(Debug, Clone, Copy)]
pub struct Text {
    /// The BCP 47 tag of the page (`<html lang>`).
    pub lang: &'static str,
    /// `rtl` for a right-to-left script, else `ltr`.
    pub dir: &'static str,
    /// The language's own name, for the language links.
    pub endonym: &'static str,
    /// The unsubscribe page's title.
    pub unsubscribe_title: &'static str,
    /// The unsubscribe page's question.
    pub unsubscribe_ask: &'static str,
    /// The unsubscribe button.
    pub unsubscribe_button: &'static str,
    /// After unsubscribing.
    pub unsubscribed: &'static str,
    /// An unsubscribe link that does not work.
    pub link_not_valid: &'static str,
    /// The confirm and preference pages' title for a link that does not work.
    pub link_title: &'static str,
    /// A confirm or preference link that does not work or has expired.
    pub link_expired: &'static str,
    /// Any failure.
    pub went_wrong: &'static str,
    /// The confirmation page's title.
    pub confirm_title: &'static str,
    /// The confirmation page's heading, with `{purpose}`.
    pub confirm_heading: &'static str,
    /// The confirmation button.
    pub confirm_button: &'static str,
    /// The page after confirming: its title.
    pub confirmed_title: &'static str,
    /// The page after confirming, with `{purpose}`.
    pub confirmed: &'static str,
    /// The preference page's title.
    pub preferences_title: &'static str,
    /// The preference page's heading.
    pub preferences_heading: &'static str,
    /// The button that withdraws a purpose.
    pub stop: &'static str,
    /// The button that grants a purpose.
    pub start: &'static str,
    /// A purpose that is granted.
    pub receiving: &'static str,
    /// A purpose that waits for confirmation.
    pub pending: &'static str,
    /// A purpose that is not granted.
    pub not_receiving: &'static str,
    /// A choice the form does not offer.
    pub not_available: &'static str,
    /// The label of the language links.
    pub other_languages: &'static str,
    /// The purposes' words: marketing, newsletter, events, product updates, any other.
    pub purposes: [&'static str; 5],
    /// The web form's confirmation mail: its subject (CRM-T180).
    pub mail_subject: &'static str,
    /// The confirmation mail's text, with `{name}`, `{purpose}`, `{link}` and `{days}`.
    pub mail_body: &'static str,
    /// The web form's answer to every submission it accepts or ignores.
    pub form_received: &'static str,
}

/// The pages' languages; English first, as the fallback.
pub const TEXTS: &[Text] = &[
    Text {
        lang: "en",
        dir: "ltr",
        endonym: "English",
        unsubscribe_title: "Unsubscribe",
        unsubscribe_ask: "Stop receiving marketing email?",
        unsubscribe_button: "Unsubscribe",
        unsubscribed: "You are unsubscribed.",
        link_not_valid: "This link is not valid.",
        link_title: "Link not valid",
        link_expired: "This link is not valid or has expired.",
        went_wrong: "Something went wrong. Please try again.",
        confirm_title: "Confirm",
        confirm_heading: "Confirm: {purpose}",
        confirm_button: "Yes, send me these",
        confirmed_title: "Confirmed",
        confirmed: "Thank you. You will receive: {purpose}.",
        preferences_title: "Your preferences",
        preferences_heading: "What you would like to receive",
        stop: "Stop",
        start: "Start",
        receiving: "You receive these.",
        pending: "Waiting for you to confirm.",
        not_receiving: "You do not receive these.",
        not_available: "That choice is not available.",
        other_languages: "Other languages",
        purposes: [
            "Marketing messages",
            "The newsletter",
            "Invitations to events",
            "Product updates",
            "Messages",
        ],
        mail_subject: "Please confirm your subscription",
        mail_body: "Hello {name},\n\nSomeone asked for this address to receive: {purpose}.\nTo confirm, open this link and press the button:\n\n{link}\n\nThe link works for {days} days. If you did not ask for this, ignore this message and nothing will be sent to you.\n",
        form_received: "If the address is valid, a confirmation message is on its way.",
    },
    Text {
        lang: "ar",
        dir: "rtl",
        endonym: "العربية",
        unsubscribe_title: "إلغاء الاشتراك",
        unsubscribe_ask: "هل تريد التوقف عن تلقي رسائل التسويق؟",
        unsubscribe_button: "إلغاء الاشتراك",
        unsubscribed: "تم إلغاء اشتراكك.",
        link_not_valid: "هذا الرابط غير صالح.",
        link_title: "الرابط غير صالح",
        link_expired: "هذا الرابط غير صالح أو انتهت صلاحيته.",
        went_wrong: "حدث خطأ ما. يُرجى المحاولة مرة أخرى.",
        confirm_title: "تأكيد",
        confirm_heading: "تأكيد: {purpose}",
        confirm_button: "نعم، أرسلوها إليّ",
        confirmed_title: "تم التأكيد",
        confirmed: "شكرًا لك. ستتلقى: {purpose}.",
        preferences_title: "تفضيلاتك",
        preferences_heading: "ما الذي تود تلقيه",
        stop: "إيقاف",
        start: "بدء",
        receiving: "أنت تتلقى هذه.",
        pending: "في انتظار تأكيدك.",
        not_receiving: "أنت لا تتلقى هذه.",
        not_available: "هذا الخيار غير متاح.",
        other_languages: "لغات أخرى",
        purposes: [
            "رسائل التسويق",
            "النشرة الإخبارية",
            "دعوات إلى الفعاليات",
            "تحديثات المنتجات",
            "الرسائل",
        ],
        mail_subject: "يرجى تأكيد اشتراكك",
        mail_body: "مرحبًا {name}،\n\nطلب شخص ما أن يتلقى هذا العنوان: {purpose}.\nللتأكيد، افتح هذا الرابط واضغط على الزر:\n\n{link}\n\nيعمل الرابط مدة {days} أيام. إذا لم تطلب ذلك، فتجاهل هذه الرسالة ولن يُرسل إليك شيء.\n",
        form_received: "إذا كان العنوان صالحًا، فإن رسالة التأكيد في طريقها إليك.",
    },
    Text {
        lang: "cy",
        dir: "ltr",
        endonym: "Cymraeg",
        unsubscribe_title: "Dad-danysgrifio",
        unsubscribe_ask: "Peidio â derbyn e-bost marchnata?",
        unsubscribe_button: "Dad-danysgrifio",
        unsubscribed: "Rydych wedi dad-danysgrifio.",
        link_not_valid: "Nid yw'r ddolen hon yn ddilys.",
        link_title: "Dolen annilys",
        link_expired: "Nid yw'r ddolen hon yn ddilys neu mae wedi dod i ben.",
        went_wrong: "Aeth rhywbeth o'i le. Rhowch gynnig arall arni.",
        confirm_title: "Cadarnhau",
        confirm_heading: "Cadarnhau: {purpose}",
        confirm_button: "Ie, anfonwch y rhain ataf",
        confirmed_title: "Wedi cadarnhau",
        confirmed: "Diolch. Byddwch yn derbyn: {purpose}.",
        preferences_title: "Eich dewisiadau",
        preferences_heading: "Beth hoffech ei dderbyn",
        stop: "Stopio",
        start: "Dechrau",
        receiving: "Rydych yn derbyn y rhain.",
        pending: "Yn aros i chi gadarnhau.",
        not_receiving: "Nid ydych yn derbyn y rhain.",
        not_available: "Nid yw'r dewis hwnnw ar gael.",
        other_languages: "Ieithoedd eraill",
        purposes: [
            "Negeseuon marchnata",
            "Y cylchlythyr",
            "Gwahoddiadau i ddigwyddiadau",
            "Newyddion am gynhyrchion",
            "Negeseuon",
        ],
        mail_subject: "Cadarnhewch eich tanysgrifiad",
        mail_body: "Helo {name},\n\nGofynnodd rhywun i'r cyfeiriad hwn dderbyn: {purpose}.\nI gadarnhau, agorwch y ddolen hon a gwasgwch y botwm:\n\n{link}\n\nMae'r ddolen yn gweithio am {days} diwrnod. Os na wnaethoch ofyn am hyn, anwybyddwch y neges hon ac ni fydd dim yn cael ei anfon atoch.\n",
        form_received: "Os yw'r cyfeiriad yn ddilys, mae neges gadarnhau ar ei ffordd.",
    },
    Text {
        lang: "de",
        dir: "ltr",
        endonym: "Deutsch",
        unsubscribe_title: "Abmelden",
        unsubscribe_ask: "Keine Marketing-E-Mails mehr erhalten?",
        unsubscribe_button: "Abmelden",
        unsubscribed: "Sie sind abgemeldet.",
        link_not_valid: "Dieser Link ist ungültig.",
        link_title: "Link ungültig",
        link_expired: "Dieser Link ist ungültig oder abgelaufen.",
        went_wrong: "Etwas ist schiefgelaufen. Bitte versuchen Sie es erneut.",
        confirm_title: "Bestätigen",
        confirm_heading: "Bestätigen: {purpose}",
        confirm_button: "Ja, bitte zusenden",
        confirmed_title: "Bestätigt",
        confirmed: "Vielen Dank. Sie erhalten: {purpose}.",
        preferences_title: "Ihre Einstellungen",
        preferences_heading: "Was Sie erhalten möchten",
        stop: "Beenden",
        start: "Starten",
        receiving: "Sie erhalten diese.",
        pending: "Wartet auf Ihre Bestätigung.",
        not_receiving: "Sie erhalten diese nicht.",
        not_available: "Diese Auswahl ist nicht verfügbar.",
        other_languages: "Andere Sprachen",
        purposes: [
            "Marketing-Nachrichten",
            "Der Newsletter",
            "Einladungen zu Veranstaltungen",
            "Produktneuigkeiten",
            "Nachrichten",
        ],
        mail_subject: "Bitte bestätigen Sie Ihr Abonnement",
        mail_body: "Hallo {name},\n\njemand hat angefragt, dass diese Adresse Folgendes erhält: {purpose}.\nZum Bestätigen öffnen Sie diesen Link und drücken Sie die Schaltfläche:\n\n{link}\n\nDer Link ist {days} Tage gültig. Wenn Sie das nicht angefragt haben, ignorieren Sie diese Nachricht; Ihnen wird dann nichts geschickt.\n",
        form_received: "Wenn die Adresse gültig ist, ist eine Bestätigungsnachricht unterwegs.",
    },
    Text {
        lang: "es",
        dir: "ltr",
        endonym: "Español",
        unsubscribe_title: "Darse de baja",
        unsubscribe_ask: "¿Dejar de recibir correos de marketing?",
        unsubscribe_button: "Darse de baja",
        unsubscribed: "Se ha dado de baja.",
        link_not_valid: "Este enlace no es válido.",
        link_title: "Enlace no válido",
        link_expired: "Este enlace no es válido o ha caducado.",
        went_wrong: "Algo salió mal. Inténtelo de nuevo.",
        confirm_title: "Confirmar",
        confirm_heading: "Confirmar: {purpose}",
        confirm_button: "Sí, envíenmelos",
        confirmed_title: "Confirmado",
        confirmed: "Gracias. Recibirá: {purpose}.",
        preferences_title: "Sus preferencias",
        preferences_heading: "Qué le gustaría recibir",
        stop: "Detener",
        start: "Empezar",
        receiving: "Los recibe.",
        pending: "Pendiente de su confirmación.",
        not_receiving: "No los recibe.",
        not_available: "Esa opción no está disponible.",
        other_languages: "Otros idiomas",
        purposes: [
            "Mensajes de marketing",
            "El boletín",
            "Invitaciones a eventos",
            "Novedades de productos",
            "Mensajes",
        ],
        mail_subject: "Confirme su suscripción",
        mail_body: "Hola, {name}:\n\nAlguien pidió que esta dirección reciba: {purpose}.\nPara confirmarlo, abra este enlace y pulse el botón:\n\n{link}\n\nEl enlace funciona durante {days} días. Si usted no lo pidió, ignore este mensaje y no se le enviará nada.\n",
        form_received: "Si la dirección es válida, un mensaje de confirmación está en camino.",
    },
    Text {
        lang: "fr",
        dir: "ltr",
        endonym: "Français",
        unsubscribe_title: "Se désabonner",
        unsubscribe_ask: "Ne plus recevoir d’e-mails marketing ?",
        unsubscribe_button: "Se désabonner",
        unsubscribed: "Vous êtes désabonné.",
        link_not_valid: "Ce lien n’est pas valide.",
        link_title: "Lien non valide",
        link_expired: "Ce lien n’est pas valide ou a expiré.",
        went_wrong: "Une erreur s’est produite. Veuillez réessayer.",
        confirm_title: "Confirmer",
        confirm_heading: "Confirmer : {purpose}",
        confirm_button: "Oui, envoyez-les-moi",
        confirmed_title: "Confirmé",
        confirmed: "Merci. Vous recevrez : {purpose}.",
        preferences_title: "Vos préférences",
        preferences_heading: "Ce que vous souhaitez recevoir",
        stop: "Arrêter",
        start: "Commencer",
        receiving: "Vous les recevez.",
        pending: "En attente de votre confirmation.",
        not_receiving: "Vous ne les recevez pas.",
        not_available: "Ce choix n’est pas disponible.",
        other_languages: "Autres langues",
        purposes: [
            "Messages marketing",
            "La lettre d’information",
            "Invitations à des événements",
            "Nouveautés produits",
            "Messages",
        ],
        mail_subject: "Veuillez confirmer votre abonnement",
        mail_body: "Bonjour {name},\n\nQuelqu’un a demandé que cette adresse reçoive : {purpose}.\nPour confirmer, ouvrez ce lien et appuyez sur le bouton :\n\n{link}\n\nLe lien fonctionne pendant {days} jours. Si vous n’avez rien demandé, ignorez ce message et rien ne vous sera envoyé.\n",
        form_received: "Si l’adresse est valide, un message de confirmation est en route.",
    },
    Text {
        lang: "hi",
        dir: "ltr",
        endonym: "हिन्दी",
        unsubscribe_title: "सदस्यता छोड़ें",
        unsubscribe_ask: "मार्केटिंग ईमेल पाना बंद करें?",
        unsubscribe_button: "सदस्यता छोड़ें",
        unsubscribed: "आपकी सदस्यता छोड़ दी गई है।",
        link_not_valid: "यह लिंक मान्य नहीं है।",
        link_title: "लिंक मान्य नहीं",
        link_expired: "यह लिंक मान्य नहीं है या इसकी अवधि समाप्त हो गई है।",
        went_wrong: "कुछ गलत हो गया। कृपया फिर से प्रयास करें।",
        confirm_title: "पुष्टि करें",
        confirm_heading: "पुष्टि करें: {purpose}",
        confirm_button: "हाँ, मुझे ये भेजें",
        confirmed_title: "पुष्टि हो गई",
        confirmed: "धन्यवाद। आपको मिलेंगे: {purpose}।",
        preferences_title: "आपकी प्राथमिकताएँ",
        preferences_heading: "आप क्या पाना चाहेंगे",
        stop: "बंद करें",
        start: "शुरू करें",
        receiving: "आपको ये मिलते हैं।",
        pending: "आपकी पुष्टि की प्रतीक्षा है।",
        not_receiving: "आपको ये नहीं मिलते।",
        not_available: "यह विकल्प उपलब्ध नहीं है।",
        other_languages: "अन्य भाषाएँ",
        purposes: [
            "मार्केटिंग संदेश",
            "न्यूज़लेटर",
            "कार्यक्रमों के निमंत्रण",
            "उत्पाद अपडेट",
            "संदेश",
        ],
        mail_subject: "कृपया अपनी सदस्यता की पुष्टि करें",
        mail_body: "नमस्ते {name},\n\nकिसी ने अनुरोध किया है कि इस पते पर यह भेजा जाए: {purpose}।\nपुष्टि करने के लिए यह लिंक खोलें और बटन दबाएँ:\n\n{link}\n\nयह लिंक {days} दिनों तक काम करता है। यदि आपने यह अनुरोध नहीं किया था, तो इस संदेश को अनदेखा करें और आपको कुछ नहीं भेजा जाएगा।\n",
        form_received: "यदि पता मान्य है, तो पुष्टि का संदेश आपके पास आ रहा है।",
    },
    Text {
        lang: "zh",
        dir: "ltr",
        endonym: "中文",
        unsubscribe_title: "退订",
        unsubscribe_ask: "不再接收营销邮件？",
        unsubscribe_button: "退订",
        unsubscribed: "您已退订。",
        link_not_valid: "此链接无效。",
        link_title: "链接无效",
        link_expired: "此链接无效或已过期。",
        went_wrong: "出了点问题，请重试。",
        confirm_title: "确认",
        confirm_heading: "确认：{purpose}",
        confirm_button: "是的，请发送给我",
        confirmed_title: "已确认",
        confirmed: "谢谢。您将收到：{purpose}。",
        preferences_title: "您的偏好",
        preferences_heading: "您希望收到哪些内容",
        stop: "停止",
        start: "开始",
        receiving: "您正在接收这些内容。",
        pending: "等待您的确认。",
        not_receiving: "您没有接收这些内容。",
        not_available: "该选项不可用。",
        other_languages: "其他语言",
        purposes: ["营销消息", "新闻通讯", "活动邀请", "产品更新", "消息"],
        mail_subject: "请确认您的订阅",
        mail_body: "{name}，您好：\n\n有人请求让此地址接收：{purpose}。\n如需确认，请打开以下链接并点击按钮：\n\n{link}\n\n该链接在 {days} 天内有效。如果您没有提出此请求，请忽略本邮件，我们不会向您发送任何内容。\n",
        form_received: "如果该地址有效，确认邮件正在发送中。",
    },
];

/// The web form's confirmation mail in `text`'s language: its subject and text. `{name}` is
/// filled last, so a name that holds a placeholder stays as typed.
#[must_use]
pub fn confirmation_mail(
    text: &Text,
    name: &str,
    purpose: &str,
    link: &str,
    days: i64,
) -> (String, String) {
    let body = text
        .mail_body
        .replace("{purpose}", text.purpose(purpose))
        .replace("{link}", link)
        .replace("{days}", &days.to_string())
        .replace("{name}", name);
    (text.mail_subject.to_string(), body)
}

/// The purposes in the order of [`Text::purposes`].
const PURPOSE_ORDER: [&str; 4] = ["marketing", "newsletter", "events", "product_updates"];

impl Text {
    /// The words for `purpose`.
    #[must_use]
    pub fn purpose(&self, purpose: &str) -> &'static str {
        let index = PURPOSE_ORDER
            .iter()
            .position(|p| *p == purpose)
            .unwrap_or(PURPOSE_ORDER.len());
        self.purposes[index]
    }
}

/// The text for a primary language subtag, if the pages have it.
#[must_use]
pub fn text_for(tag: &str) -> Option<&'static Text> {
    let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    TEXTS.iter().find(|t| t.lang == primary)
}

/// The English text, the fallback.
#[must_use]
pub fn english() -> &'static Text {
    &TEXTS[0]
}

/// The page's language: `chosen` (the address's `?lang=`) if the pages have it, else the best
/// language of `accept_language` they have, by its `q` weights in the order given, else English.
/// A `*` or a language with `q=0` chooses nothing.
#[must_use]
pub fn negotiate(chosen: Option<&str>, accept_language: Option<&str>) -> &'static Text {
    if let Some(text) = chosen.and_then(text_for) {
        return text;
    }
    let mut ranges: Vec<(u16, usize, &str)> = Vec::new();
    for (position, item) in accept_language.unwrap_or_default().split(',').enumerate() {
        let mut parts = item.split(';');
        let range = parts.next().unwrap_or_default().trim();
        let weight = parts
            .find_map(|p| p.trim().strip_prefix("q="))
            .map_or(1000, quality);
        if !range.is_empty() && range != "*" && weight > 0 {
            ranges.push((weight, position, range));
        }
    }
    ranges.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranges
        .iter()
        .find_map(|(_, _, range)| text_for(range))
        .unwrap_or_else(english)
}

/// A `q` value (0 to 1, up to three decimals) in thousandths; anything else counts as 0.
fn quality(text: &str) -> u16 {
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    let thousandths = format!("{fraction:0<3}").parse::<u16>().unwrap_or(0);
    match whole {
        "0" => thousandths,
        "1" if thousandths == 0 => 1000,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_s_language_is_chosen_by_weight_then_order() {
        assert_eq!(negotiate(None, Some("de-AT,de;q=0.9,en;q=0.5")).lang, "de");
        assert_eq!(
            negotiate(None, Some("ja, zh-CN;q=0.8, en;q=0.7")).lang,
            "zh"
        );
        assert_eq!(negotiate(None, Some("en;q=0.2, ar;q=0.9")).lang, "ar");
        assert_eq!(negotiate(None, Some("cy, fr")).lang, "cy");
        assert_eq!(negotiate(None, Some("fr;q=0, hi")).lang, "hi");
        assert_eq!(
            negotiate(None, Some("ja, ko")).lang,
            "en",
            "none of ours: English"
        );
        assert_eq!(negotiate(None, Some("*")).lang, "en");
        assert_eq!(negotiate(None, None).lang, "en");
        assert_eq!(
            negotiate(None, Some("es;q=1.5, fr;q=0.4")).lang,
            "fr",
            "a bad q is 0"
        );
    }

    #[test]
    fn the_address_s_choice_wins() {
        assert_eq!(negotiate(Some("fr"), Some("de")).lang, "fr");
        assert_eq!(
            negotiate(Some("pt-BR"), Some("de")).lang,
            "de",
            "a language we lack"
        );
        assert_eq!(negotiate(Some("ZH_hans"), None).lang, "zh");
    }

    #[test]
    fn every_language_is_complete_and_its_own() {
        let english = english();
        for text in TEXTS {
            let all = [
                text.unsubscribe_title,
                text.unsubscribe_ask,
                text.unsubscribe_button,
                text.unsubscribed,
                text.link_not_valid,
                text.link_title,
                text.link_expired,
                text.went_wrong,
                text.confirm_title,
                text.confirm_heading,
                text.confirm_button,
                text.confirmed_title,
                text.confirmed,
                text.preferences_title,
                text.preferences_heading,
                text.stop,
                text.start,
                text.receiving,
                text.pending,
                text.not_receiving,
                text.not_available,
                text.other_languages,
                text.mail_subject,
                text.mail_body,
                text.form_received,
            ];
            assert!(
                all.iter()
                    .chain(text.purposes.iter())
                    .all(|s| !s.trim().is_empty())
            );
            assert!(
                text.confirm_heading.contains("{purpose}") && text.confirmed.contains("{purpose}")
            );
            for placeholder in ["{name}", "{purpose}", "{link}", "{days}"] {
                assert_eq!(
                    text.mail_body.matches(placeholder).count(),
                    1,
                    "{}: {placeholder}",
                    text.lang
                );
            }
            assert!(
                text.mail_body.contains("\n{link}\n"),
                "{}: the link on its own line",
                text.lang
            );
            assert!(
                all.iter().all(|s| !s.contains(['<', '>', '&', '"'])),
                "{}: no markup",
                text.lang
            );
            if text.lang != "en" {
                assert_ne!(
                    text.unsubscribe_ask, english.unsubscribe_ask,
                    "{}",
                    text.lang
                );
                assert_ne!(text.purposes, english.purposes, "{}", text.lang);
                assert_ne!(text.mail_subject, english.mail_subject, "{}", text.lang);
                assert_ne!(text.mail_body, english.mail_body, "{}", text.lang);
                assert_ne!(text.form_received, english.form_received, "{}", text.lang);
            }
            assert_eq!(text.dir == "rtl", text.lang == "ar");
        }
        assert_eq!(TEXTS.len(), 8, "the UI's eight languages");
    }

    #[test]
    fn purposes_have_their_words() {
        let de = text_for("de").unwrap();
        assert_eq!(de.purpose("newsletter"), "Der Newsletter");
        assert_eq!(de.purpose("something_else"), "Nachrichten");
        assert_eq!(english().purpose("marketing"), "Marketing messages");
    }

    #[test]
    fn the_confirmation_mail_is_filled_in_its_language() {
        let (subject, body) = confirmation_mail(
            text_for("de").unwrap(),
            "Ada {link}",
            "newsletter",
            "https://crm.test/confirm/t?lang=de",
            7,
        );
        assert_eq!(subject, "Bitte bestätigen Sie Ihr Abonnement");
        assert!(body.starts_with("Hallo Ada {link},"), "{body}");
        assert!(body.contains("Der Newsletter"), "{body}");
        assert!(
            body.contains("\nhttps://crm.test/confirm/t?lang=de\n"),
            "{body}"
        );
        assert!(body.contains("7 Tage"), "{body}");
        let (subject, body) = confirmation_mail(english(), "Ada", "marketing", "L", 7);
        assert_eq!(subject, "Please confirm your subscription");
        assert!(body.contains("receive: Marketing messages."), "{body}");
    }
}
