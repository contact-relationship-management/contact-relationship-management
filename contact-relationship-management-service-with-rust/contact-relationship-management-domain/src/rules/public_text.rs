//! The words of the public consent pages, by language (CRM-T175, CRM-R29, CRM-R95).
//!
//! The confirmation, preference and unsubscribe pages are what a data subject reads about their
//! own consent, so they are in the reader's language: the one the reader's browser asks for
//! (`Accept-Language`, RFC 9110 §12.5.4, with its `q` weights), unless the page's address names
//! one (`?lang=`, from the language links every page carries). The contact's language is not
//! stored: the browser says it on every visit, and storing it would be one more piece of personal
//! data kept for no other use. A language not here falls back to English, which is recorded as a
//! limit. The 21 languages are the UI's; every one but English is a machine draft by the AI
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
    // The web form's refusals (CRM-T182): its `422`, `403`, `429` and `503` answers' title and
    // text. The problem document's `error` code stays the same in every language.
    /// The title of the web form's `422` answer (CRM-T182).
    pub form_invalid_title: &'static str,
    /// The web form's name that cannot be used.
    pub form_name_invalid: &'static str,
    /// The web form's address that cannot be used.
    pub form_email_invalid: &'static str,
    /// The title of the web form's `403` answer (another website, or none).
    pub form_refused_title: &'static str,
    /// The web form's `403` answer.
    pub form_refused: &'static str,
    /// The title of the web form's `429` answer.
    pub form_busy_title: &'static str,
    /// The web form's `429` answer.
    pub form_busy: &'static str,
    /// The title of the web form's `503` answer.
    pub form_unavailable_title: &'static str,
    /// The web form's `503` answer.
    pub form_unavailable: &'static str,
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
        form_invalid_title: "Check the form",
        form_name_invalid: "Please enter a name of at most 200 characters, on one line.",
        form_email_invalid: "Please enter an email address that can receive mail.",
        form_refused_title: "Not allowed",
        form_refused: "This form cannot be sent from this website.",
        form_busy_title: "Too many submissions",
        form_busy: "Too many submissions. Please wait a little and try again.",
        form_unavailable_title: "Not available",
        form_unavailable: "The form cannot be sent just now. Please try again later.",
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
        form_invalid_title: "تحقق من النموذج",
        form_name_invalid: "يُرجى إدخال اسم لا يزيد على 200 حرف، في سطر واحد.",
        form_email_invalid: "يُرجى إدخال عنوان بريد إلكتروني يمكنه تلقي الرسائل.",
        form_refused_title: "غير مسموح",
        form_refused: "لا يمكن إرسال هذا النموذج من هذا الموقع.",
        form_busy_title: "عدد كبير جدًا من الطلبات",
        form_busy: "عدد كبير جدًا من الطلبات. يُرجى الانتظار قليلًا والمحاولة مرة أخرى.",
        form_unavailable_title: "غير متاح",
        form_unavailable: "لا يمكن إرسال النموذج الآن. يُرجى المحاولة لاحقًا.",
    },
    Text {
        lang: "bn",
        dir: "ltr",
        endonym: "বাংলা",
        unsubscribe_title: "সদস্যতা বাতিল",
        unsubscribe_ask: "বিপণনের ইমেল পাওয়া বন্ধ করতে চান?",
        unsubscribe_button: "সদস্যতা বাতিল করুন",
        unsubscribed: "আপনার সদস্যতা বাতিল করা হয়েছে।",
        link_not_valid: "এই লিংকটি সঠিক নয়।",
        link_title: "লিংক সঠিক নয়",
        link_expired: "এই লিংকটি সঠিক নয় অথবা মেয়াদ শেষ হয়েছে।",
        went_wrong: "কিছু একটা ভুল হয়েছে। অনুগ্রহ করে আবার চেষ্টা করুন।",
        confirm_title: "নিশ্চিত করুন",
        confirm_heading: "নিশ্চিত করুন: {purpose}",
        confirm_button: "হ্যাঁ, এগুলো আমাকে পাঠান",
        confirmed_title: "নিশ্চিত হয়েছে",
        confirmed: "ধন্যবাদ। আপনি পাবেন: {purpose}।",
        preferences_title: "আপনার পছন্দ",
        preferences_heading: "আপনি কী কী পেতে চান",
        stop: "বন্ধ করুন",
        start: "শুরু করুন",
        receiving: "আপনি এগুলো পান।",
        pending: "আপনার নিশ্চিতকরণের অপেক্ষায়।",
        not_receiving: "আপনি এগুলো পান না।",
        not_available: "এই বিকল্পটি উপলব্ধ নয়।",
        other_languages: "অন্যান্য ভাষা",
        purposes: [
            "বিপণনের বার্তা",
            "নিউজলেটার",
            "অনুষ্ঠানের আমন্ত্রণ",
            "পণ্যের হালনাগাদ",
            "বার্তা",
        ],
        mail_subject: "অনুগ্রহ করে আপনার সদস্যতা নিশ্চিত করুন",
        mail_body: "প্রিয় {name},\n\nকেউ এই ঠিকানায় এগুলো পাঠানোর অনুরোধ করেছেন: {purpose}।\nনিশ্চিত করতে এই লিংকটি খুলুন এবং বোতামটি চাপুন:\n\n{link}\n\nলিংকটি {days} দিন কাজ করবে। আপনি এই অনুরোধ না করে থাকলে এই বার্তাটি উপেক্ষা করুন; আপনাকে কিছু পাঠানো হবে না।\n",
        form_received: "ঠিকানাটি সঠিক হলে একটি নিশ্চিতকরণ বার্তা পাঠানো হচ্ছে।",
        form_invalid_title: "ফর্মটি পরীক্ষা করুন",
        form_name_invalid: "অনুগ্রহ করে এক লাইনে সর্বোচ্চ ২০০ অক্ষরের একটি নাম লিখুন।",
        form_email_invalid: "অনুগ্রহ করে এমন একটি ইমেল ঠিকানা লিখুন যেখানে বার্তা গ্রহণ করা যায়।",
        form_refused_title: "অনুমোদিত নয়",
        form_refused: "এই ওয়েবসাইট থেকে এই ফর্ম পাঠানো যাবে না।",
        form_busy_title: "অতিরিক্ত অনুরোধ",
        form_busy: "অতিরিক্ত অনুরোধ এসেছে। অনুগ্রহ করে একটু অপেক্ষা করে আবার চেষ্টা করুন।",
        form_unavailable_title: "উপলব্ধ নয়",
        form_unavailable: "ফর্মটি এখন পাঠানো যাচ্ছে না। অনুগ্রহ করে পরে আবার চেষ্টা করুন।",
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
        form_invalid_title: "Gwiriwch y ffurflen",
        form_name_invalid: "Rhowch enw o 200 nod ar y mwyaf, ar un llinell.",
        form_email_invalid: "Rhowch gyfeiriad e-bost sy'n gallu derbyn post.",
        form_refused_title: "Ni chaniateir",
        form_refused: "Ni ellir anfon y ffurflen hon o'r wefan hon.",
        form_busy_title: "Gormod o gyflwyniadau",
        form_busy: "Gormod o gyflwyniadau. Arhoswch ychydig a rhowch gynnig arall arni.",
        form_unavailable_title: "Ddim ar gael",
        form_unavailable: "Ni ellir anfon y ffurflen ar hyn o bryd. Rhowch gynnig arall arni yn nes ymlaen.",
    },
    Text {
        lang: "da",
        dir: "ltr",
        endonym: "Dansk",
        unsubscribe_title: "Afmeld",
        unsubscribe_ask: "Vil du ikke længere modtage marketingmails?",
        unsubscribe_button: "Afmeld",
        unsubscribed: "Du er afmeldt.",
        link_not_valid: "Dette link er ikke gyldigt.",
        link_title: "Linket er ikke gyldigt",
        link_expired: "Dette link er ikke gyldigt eller er udløbet.",
        went_wrong: "Noget gik galt. Prøv igen.",
        confirm_title: "Bekræft",
        confirm_heading: "Bekræft: {purpose}",
        confirm_button: "Ja, send mig dette",
        confirmed_title: "Bekræftet",
        confirmed: "Tak. Du modtager: {purpose}.",
        preferences_title: "Dine indstillinger",
        preferences_heading: "Hvad du ønsker at modtage",
        stop: "Stop",
        start: "Start",
        receiving: "Du modtager dette.",
        pending: "Venter på din bekræftelse.",
        not_receiving: "Du modtager ikke dette.",
        not_available: "Dette valg er ikke tilgængeligt.",
        other_languages: "Andre sprog",
        purposes: [
            "Marketingbeskeder",
            "Nyhedsbrevet",
            "Invitationer til arrangementer",
            "Produktnyheder",
            "Beskeder",
        ],
        mail_subject: "Bekræft venligst dit abonnement",
        mail_body: "Hej {name},\n\nNogen har bedt om, at denne adresse modtager: {purpose}.\nFor at bekræfte skal du åbne dette link og trykke på knappen:\n\n{link}\n\nLinket virker i {days} dage. Hvis du ikke har bedt om dette, kan du se bort fra denne besked, og så sendes der intet til dig.\n",
        form_received: "Hvis adressen er gyldig, er en bekræftelsesbesked på vej.",
        form_invalid_title: "Tjek formularen",
        form_name_invalid: "Indtast et navn på højst 200 tegn på én linje.",
        form_email_invalid: "Indtast en e-mailadresse, der kan modtage post.",
        form_refused_title: "Ikke tilladt",
        form_refused: "Denne formular kan ikke sendes fra dette websted.",
        form_busy_title: "For mange forsøg",
        form_busy: "For mange forsøg. Vent lidt, og prøv igen.",
        form_unavailable_title: "Ikke tilgængelig",
        form_unavailable: "Formularen kan ikke sendes lige nu. Prøv igen senere.",
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
        form_invalid_title: "Formular prüfen",
        form_name_invalid: "Bitte geben Sie einen Namen mit höchstens 200 Zeichen in einer Zeile ein.",
        form_email_invalid: "Bitte geben Sie eine E-Mail-Adresse ein, die Nachrichten empfangen kann.",
        form_refused_title: "Nicht erlaubt",
        form_refused: "Dieses Formular kann von dieser Website aus nicht gesendet werden.",
        form_busy_title: "Zu viele Anfragen",
        form_busy: "Zu viele Anfragen. Bitte warten Sie kurz und versuchen Sie es erneut.",
        form_unavailable_title: "Nicht verfügbar",
        form_unavailable: "Das Formular kann gerade nicht gesendet werden. Bitte versuchen Sie es später erneut.",
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
        form_invalid_title: "Revise el formulario",
        form_name_invalid: "Introduzca un nombre de 200 caracteres como máximo, en una sola línea.",
        form_email_invalid: "Introduzca una dirección de correo electrónico que pueda recibir mensajes.",
        form_refused_title: "No permitido",
        form_refused: "Este formulario no se puede enviar desde este sitio web.",
        form_busy_title: "Demasiados envíos",
        form_busy: "Demasiados envíos. Espere un poco y vuelva a intentarlo.",
        form_unavailable_title: "No disponible",
        form_unavailable: "El formulario no se puede enviar en este momento. Vuelva a intentarlo más tarde.",
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
        form_invalid_title: "Vérifiez le formulaire",
        form_name_invalid: "Veuillez saisir un nom de 200 caractères au plus, sur une seule ligne.",
        form_email_invalid: "Veuillez saisir une adresse e-mail qui peut recevoir des messages.",
        form_refused_title: "Non autorisé",
        form_refused: "Ce formulaire ne peut pas être envoyé depuis ce site.",
        form_busy_title: "Trop d’envois",
        form_busy: "Trop d’envois. Veuillez patienter un peu et réessayer.",
        form_unavailable_title: "Indisponible",
        form_unavailable: "Le formulaire ne peut pas être envoyé pour le moment. Veuillez réessayer plus tard.",
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
        form_invalid_title: "फ़ॉर्म जाँचें",
        form_name_invalid: "कृपया अधिकतम 200 अक्षरों का नाम, एक ही पंक्ति में, दर्ज करें।",
        form_email_invalid: "कृपया ऐसा ईमेल पता दर्ज करें जिस पर संदेश प्राप्त हो सकें।",
        form_refused_title: "अनुमति नहीं है",
        form_refused: "यह फ़ॉर्म इस वेबसाइट से नहीं भेजा जा सकता।",
        form_busy_title: "बहुत अधिक अनुरोध",
        form_busy: "बहुत अधिक अनुरोध। कृपया थोड़ा रुककर फिर से प्रयास करें।",
        form_unavailable_title: "उपलब्ध नहीं",
        form_unavailable: "फ़ॉर्म अभी नहीं भेजा जा सकता। कृपया बाद में फिर से प्रयास करें।",
    },
    Text {
        lang: "id",
        dir: "ltr",
        endonym: "Bahasa Indonesia",
        unsubscribe_title: "Berhenti berlangganan",
        unsubscribe_ask: "Berhenti menerima email pemasaran?",
        unsubscribe_button: "Berhenti berlangganan",
        unsubscribed: "Anda sudah berhenti berlangganan.",
        link_not_valid: "Tautan ini tidak valid.",
        link_title: "Tautan tidak valid",
        link_expired: "Tautan ini tidak valid atau sudah kedaluwarsa.",
        went_wrong: "Terjadi kesalahan. Silakan coba lagi.",
        confirm_title: "Konfirmasi",
        confirm_heading: "Konfirmasi: {purpose}",
        confirm_button: "Ya, kirimkan kepada saya",
        confirmed_title: "Terkonfirmasi",
        confirmed: "Terima kasih. Anda akan menerima: {purpose}.",
        preferences_title: "Preferensi Anda",
        preferences_heading: "Yang ingin Anda terima",
        stop: "Hentikan",
        start: "Mulai",
        receiving: "Anda menerima ini.",
        pending: "Menunggu konfirmasi Anda.",
        not_receiving: "Anda tidak menerima ini.",
        not_available: "Pilihan itu tidak tersedia.",
        other_languages: "Bahasa lain",
        purposes: [
            "Pesan pemasaran",
            "Buletin",
            "Undangan acara",
            "Pembaruan produk",
            "Pesan",
        ],
        mail_subject: "Silakan konfirmasi langganan Anda",
        mail_body: "Halo {name},\n\nSeseorang meminta agar alamat ini menerima: {purpose}.\nUntuk mengonfirmasi, buka tautan ini dan tekan tombolnya:\n\n{link}\n\nTautan ini berlaku selama {days} hari. Jika Anda tidak memintanya, abaikan pesan ini dan tidak ada yang akan dikirim kepada Anda.\n",
        form_received: "Jika alamatnya valid, pesan konfirmasi sedang dikirim.",
        form_invalid_title: "Periksa formulir",
        form_name_invalid: "Masukkan nama paling banyak 200 karakter dalam satu baris.",
        form_email_invalid: "Masukkan alamat email yang dapat menerima pesan.",
        form_refused_title: "Tidak diizinkan",
        form_refused: "Formulir ini tidak dapat dikirim dari situs web ini.",
        form_busy_title: "Terlalu banyak pengiriman",
        form_busy: "Terlalu banyak pengiriman. Mohon tunggu sebentar lalu coba lagi.",
        form_unavailable_title: "Tidak tersedia",
        form_unavailable: "Formulir tidak dapat dikirim saat ini. Silakan coba lagi nanti.",
    },
    Text {
        lang: "it",
        dir: "ltr",
        endonym: "Italiano",
        unsubscribe_title: "Annulla l'iscrizione",
        unsubscribe_ask: "Non vuole più ricevere e-mail promozionali?",
        unsubscribe_button: "Annulla l'iscrizione",
        unsubscribed: "La sua iscrizione è stata annullata.",
        link_not_valid: "Questo link non è valido.",
        link_title: "Link non valido",
        link_expired: "Questo link non è valido oppure è scaduto.",
        went_wrong: "Si è verificato un errore. Riprovi.",
        confirm_title: "Conferma",
        confirm_heading: "Conferma: {purpose}",
        confirm_button: "Sì, desidero riceverli",
        confirmed_title: "Confermato",
        confirmed: "Grazie. Riceverà: {purpose}.",
        preferences_title: "Le sue preferenze",
        preferences_heading: "Che cosa desidera ricevere",
        stop: "Interrompi",
        start: "Attiva",
        receiving: "Li riceve.",
        pending: "In attesa della sua conferma.",
        not_receiving: "Non li riceve.",
        not_available: "Questa scelta non è disponibile.",
        other_languages: "Altre lingue",
        purposes: [
            "Messaggi promozionali",
            "La newsletter",
            "Inviti a eventi",
            "Novità sui prodotti",
            "Comunicazioni",
        ],
        mail_subject: "Confermi la sua iscrizione",
        mail_body: "Gentile {name},\n\nQualcuno ha chiesto che questo indirizzo riceva: {purpose}.\nPer confermare, apra questo link e prema il pulsante:\n\n{link}\n\nIl link è valido per {days} giorni. Se non ha fatto questa richiesta, ignori questo messaggio e non le sarà inviato nulla.\n",
        form_received: "Se l'indirizzo è valido, un messaggio di conferma è in arrivo.",
        form_invalid_title: "Controlli il modulo",
        form_name_invalid: "Inserisca un nome di al massimo 200 caratteri, su una sola riga.",
        form_email_invalid: "Inserisca un indirizzo e-mail in grado di ricevere messaggi.",
        form_refused_title: "Non consentito",
        form_refused: "Questo modulo non può essere inviato da questo sito web.",
        form_busy_title: "Troppi invii",
        form_busy: "Troppi invii. Attenda un momento e riprovi.",
        form_unavailable_title: "Non disponibile",
        form_unavailable: "Il modulo non può essere inviato in questo momento. Riprovi più tardi.",
    },
    Text {
        lang: "ja",
        dir: "ltr",
        endonym: "日本語",
        unsubscribe_title: "配信停止",
        unsubscribe_ask: "マーケティングメールの受信を停止しますか？",
        unsubscribe_button: "配信を停止する",
        unsubscribed: "配信を停止しました。",
        link_not_valid: "このリンクは無効です。",
        link_title: "無効なリンク",
        link_expired: "このリンクは無効か、有効期限が切れています。",
        went_wrong: "問題が発生しました。もう一度お試しください。",
        confirm_title: "確認",
        confirm_heading: "確認：{purpose}",
        confirm_button: "はい、受け取ります",
        confirmed_title: "確認しました",
        confirmed: "ありがとうございます。次をお届けします：{purpose}。",
        preferences_title: "受信設定",
        preferences_heading: "受け取りたい内容",
        stop: "停止",
        start: "開始",
        receiving: "受信しています。",
        pending: "確認をお待ちしています。",
        not_receiving: "受信していません。",
        not_available: "この選択肢は利用できません。",
        other_languages: "他の言語",
        purposes: [
            "マーケティングのお知らせ",
            "ニュースレター",
            "イベントのご案内",
            "製品のアップデート",
            "その他のお知らせ",
        ],
        mail_subject: "購読のご確認をお願いします",
        mail_body: "{name} 様\n\nどなたかが、このアドレスで次を受け取るよう申し込まれました：{purpose}。\n確認するには、次のリンクを開いてボタンを押してください。\n\n{link}\n\nリンクは {days} 日間有効です。お心当たりがない場合は、このメッセージを無視してください。何も送信されません。\n",
        form_received: "アドレスが有効な場合は、確認メールをお送りします。",
        form_invalid_title: "フォームをご確認ください",
        form_name_invalid: "お名前は1行で、200文字以内で入力してください。",
        form_email_invalid: "受信できるメールアドレスを入力してください。",
        form_refused_title: "許可されていません",
        form_refused: "このフォームは、このウェブサイトからは送信できません。",
        form_busy_title: "送信が多すぎます",
        form_busy: "送信が多すぎます。しばらくしてから、もう一度お試しください。",
        form_unavailable_title: "利用できません",
        form_unavailable: "現在フォームを送信できません。後でもう一度お試しください。",
    },
    Text {
        lang: "ko",
        dir: "ltr",
        endonym: "한국어",
        unsubscribe_title: "수신 거부",
        unsubscribe_ask: "마케팅 이메일 수신을 중단하시겠습니까?",
        unsubscribe_button: "수신 거부",
        unsubscribed: "수신 거부가 완료되었습니다.",
        link_not_valid: "이 링크는 유효하지 않습니다.",
        link_title: "유효하지 않은 링크",
        link_expired: "이 링크는 유효하지 않거나 만료되었습니다.",
        went_wrong: "문제가 발생했습니다. 다시 시도해 주세요.",
        confirm_title: "확인",
        confirm_heading: "확인: {purpose}",
        confirm_button: "예, 받겠습니다",
        confirmed_title: "확인되었습니다",
        confirmed: "감사합니다. 다음을 받아 보시게 됩니다: {purpose}.",
        preferences_title: "수신 설정",
        preferences_heading: "받고 싶은 내용",
        stop: "중단",
        start: "시작",
        receiving: "수신 중입니다.",
        pending: "확인을 기다리고 있습니다.",
        not_receiving: "수신하지 않습니다.",
        not_available: "이 선택은 사용할 수 없습니다.",
        other_languages: "다른 언어",
        purposes: [
            "마케팅 메시지",
            "뉴스레터",
            "행사 초대",
            "제품 업데이트",
            "기타 안내",
        ],
        mail_subject: "구독을 확인해 주세요",
        mail_body: "{name} 님께\n\n누군가 이 주소로 다음을 받도록 신청했습니다: {purpose}.\n확인하려면 다음 링크를 열고 버튼을 눌러 주세요.\n\n{link}\n\n링크는 {days}일 동안 유효합니다. 신청하신 적이 없다면 이 메시지를 무시해 주세요. 아무것도 발송되지 않습니다.\n",
        form_received: "주소가 유효하면 확인 메시지가 발송됩니다.",
        form_invalid_title: "양식을 확인해 주세요",
        form_name_invalid: "이름은 한 줄에 200자 이내로 입력해 주세요.",
        form_email_invalid: "메일을 받을 수 있는 이메일 주소를 입력해 주세요.",
        form_refused_title: "허용되지 않음",
        form_refused: "이 양식은 이 웹사이트에서 보낼 수 없습니다.",
        form_busy_title: "제출이 너무 많습니다",
        form_busy: "제출이 너무 많습니다. 잠시 기다린 후 다시 시도해 주세요.",
        form_unavailable_title: "사용할 수 없음",
        form_unavailable: "지금은 양식을 보낼 수 없습니다. 나중에 다시 시도해 주세요.",
    },
    Text {
        lang: "nl",
        dir: "ltr",
        endonym: "Nederlands",
        unsubscribe_title: "Afmelden",
        unsubscribe_ask: "Geen marketing-e-mails meer ontvangen?",
        unsubscribe_button: "Afmelden",
        unsubscribed: "U bent afgemeld.",
        link_not_valid: "Deze link is ongeldig.",
        link_title: "Link ongeldig",
        link_expired: "Deze link is ongeldig of verlopen.",
        went_wrong: "Er is iets misgegaan. Probeer het opnieuw.",
        confirm_title: "Bevestigen",
        confirm_heading: "Bevestigen: {purpose}",
        confirm_button: "Ja, stuur mij deze berichten",
        confirmed_title: "Bevestigd",
        confirmed: "Dank u wel. U ontvangt: {purpose}.",
        preferences_title: "Uw voorkeuren",
        preferences_heading: "Wat u wilt ontvangen",
        stop: "Stoppen",
        start: "Starten",
        receiving: "U ontvangt deze berichten.",
        pending: "Wacht op uw bevestiging.",
        not_receiving: "U ontvangt deze berichten niet.",
        not_available: "Deze keuze is niet beschikbaar.",
        other_languages: "Andere talen",
        purposes: [
            "Marketingberichten",
            "De nieuwsbrief",
            "Uitnodigingen voor evenementen",
            "Productnieuws",
            "Berichten",
        ],
        mail_subject: "Bevestig uw aanmelding",
        mail_body: "Hallo {name},\n\nIemand heeft gevraagd dat dit adres het volgende ontvangt: {purpose}.\nOm te bevestigen opent u deze link en drukt u op de knop:\n\n{link}\n\nDe link is {days} dagen geldig. Hebt u dit niet gevraagd, negeer dan dit bericht; er wordt dan niets naar u verstuurd.\n",
        form_received: "Als het adres geldig is, is er een bevestigingsbericht onderweg.",
        form_invalid_title: "Controleer het formulier",
        form_name_invalid: "Vul een naam in van hoogstens 200 tekens, op één regel.",
        form_email_invalid: "Vul een e-mailadres in dat berichten kan ontvangen.",
        form_refused_title: "Niet toegestaan",
        form_refused: "Dit formulier kan niet vanaf deze website worden verzonden.",
        form_busy_title: "Te veel verzoeken",
        form_busy: "Te veel verzoeken. Wacht even en probeer het opnieuw.",
        form_unavailable_title: "Niet beschikbaar",
        form_unavailable: "Het formulier kan nu niet worden verzonden. Probeer het later opnieuw.",
    },
    Text {
        lang: "pt",
        dir: "ltr",
        endonym: "Português",
        unsubscribe_title: "Cancelar inscrição",
        unsubscribe_ask: "Deixar de receber e-mails de marketing?",
        unsubscribe_button: "Cancelar inscrição",
        unsubscribed: "Sua inscrição foi cancelada.",
        link_not_valid: "Este link não é válido.",
        link_title: "Link inválido",
        link_expired: "Este link não é válido ou expirou.",
        went_wrong: "Algo deu errado. Tente novamente.",
        confirm_title: "Confirmar",
        confirm_heading: "Confirmar: {purpose}",
        confirm_button: "Sim, enviem-me estas mensagens",
        confirmed_title: "Confirmado",
        confirmed: "Obrigado. Você receberá: {purpose}.",
        preferences_title: "Suas preferências",
        preferences_heading: "O que você gostaria de receber",
        stop: "Parar",
        start: "Iniciar",
        receiving: "Você recebe estas mensagens.",
        pending: "Aguardando a sua confirmação.",
        not_receiving: "Você não recebe estas mensagens.",
        not_available: "Esta opção não está disponível.",
        other_languages: "Outros idiomas",
        purposes: [
            "Mensagens de marketing",
            "O boletim informativo",
            "Convites para eventos",
            "Novidades sobre produtos",
            "Mensagens",
        ],
        mail_subject: "Confirme a sua inscrição",
        mail_body: "Olá, {name},\n\nAlguém pediu que este endereço receba: {purpose}.\nPara confirmar, abra este link e clique no botão:\n\n{link}\n\nO link é válido por {days} dias. Se você não fez este pedido, ignore esta mensagem; nada será enviado a você.\n",
        form_received: "Se o endereço for válido, uma mensagem de confirmação está a caminho.",
        form_invalid_title: "Verifique o formulário",
        form_name_invalid: "Informe um nome com no máximo 200 caracteres, em uma só linha.",
        form_email_invalid: "Informe um endereço de e-mail que possa receber mensagens.",
        form_refused_title: "Não permitido",
        form_refused: "Este formulário não pode ser enviado a partir deste site.",
        form_busy_title: "Envios em excesso",
        form_busy: "Envios em excesso. Aguarde um pouco e tente novamente.",
        form_unavailable_title: "Indisponível",
        form_unavailable: "Não é possível enviar o formulário agora. Tente novamente mais tarde.",
    },
    Text {
        lang: "ru",
        dir: "ltr",
        endonym: "Русский",
        unsubscribe_title: "Отписаться",
        unsubscribe_ask: "Больше не получать маркетинговые письма?",
        unsubscribe_button: "Отписаться",
        unsubscribed: "Вы отписались от рассылки.",
        link_not_valid: "Эта ссылка недействительна.",
        link_title: "Ссылка недействительна",
        link_expired: "Эта ссылка недействительна или срок её действия истёк.",
        went_wrong: "Что-то пошло не так. Пожалуйста, попробуйте ещё раз.",
        confirm_title: "Подтверждение",
        confirm_heading: "Подтвердите: {purpose}",
        confirm_button: "Да, присылайте мне это",
        confirmed_title: "Подтверждено",
        confirmed: "Спасибо. Вы будете получать: {purpose}.",
        preferences_title: "Ваши настройки",
        preferences_heading: "Что вы хотели бы получать",
        stop: "Остановить",
        start: "Включить",
        receiving: "Вы получаете эти сообщения.",
        pending: "Ожидает вашего подтверждения.",
        not_receiving: "Вы не получаете эти сообщения.",
        not_available: "Этот вариант недоступен.",
        other_languages: "Другие языки",
        purposes: [
            "Маркетинговые сообщения",
            "Рассылка новостей",
            "Приглашения на мероприятия",
            "Новости о продуктах",
            "Сообщения",
        ],
        mail_subject: "Подтвердите вашу подписку",
        mail_body: "Здравствуйте, {name}!\n\nКто-то попросил, чтобы на этот адрес приходило: {purpose}.\nЧтобы подтвердить, откройте эту ссылку и нажмите кнопку:\n\n{link}\n\nСсылка действует {days} дн. Если вы не делали такого запроса, просто проигнорируйте это письмо; вам ничего не будет отправлено.\n",
        form_received: "Если адрес действителен, письмо с подтверждением уже в пути.",
        form_invalid_title: "Проверьте форму",
        form_name_invalid: "Введите имя не длиннее 200 символов, в одну строку.",
        form_email_invalid: "Введите адрес электронной почты, на который можно получать письма.",
        form_refused_title: "Не разрешено",
        form_refused: "Эту форму нельзя отправить с этого сайта.",
        form_busy_title: "Слишком много отправок",
        form_busy: "Слишком много отправок. Пожалуйста, немного подождите и попробуйте снова.",
        form_unavailable_title: "Недоступно",
        form_unavailable: "Сейчас форму отправить нельзя. Пожалуйста, попробуйте позже.",
    },
    Text {
        lang: "sv",
        dir: "ltr",
        endonym: "Svenska",
        unsubscribe_title: "Avsluta prenumeration",
        unsubscribe_ask: "Vill du sluta få marknadsföring via e-post?",
        unsubscribe_button: "Avsluta prenumeration",
        unsubscribed: "Din prenumeration är avslutad.",
        link_not_valid: "Länken är inte giltig.",
        link_title: "Ogiltig länk",
        link_expired: "Länken är inte giltig eller har gått ut.",
        went_wrong: "Något gick fel. Försök igen.",
        confirm_title: "Bekräfta",
        confirm_heading: "Bekräfta: {purpose}",
        confirm_button: "Ja, skicka dem till mig",
        confirmed_title: "Bekräftat",
        confirmed: "Tack. Du kommer att få: {purpose}.",
        preferences_title: "Dina inställningar",
        preferences_heading: "Vad du vill få",
        stop: "Stoppa",
        start: "Starta",
        receiving: "Du får dessa.",
        pending: "Väntar på din bekräftelse.",
        not_receiving: "Du får inte dessa.",
        not_available: "Det valet är inte tillgängligt.",
        other_languages: "Andra språk",
        purposes: [
            "Marknadsföringsmeddelanden",
            "Nyhetsbrevet",
            "Inbjudningar till evenemang",
            "Produktuppdateringar",
            "Meddelanden",
        ],
        mail_subject: "Bekräfta din prenumeration",
        mail_body: "Hej {name},\n\nNågon har begärt att den här adressen ska få: {purpose}.\nFör att bekräfta öppnar du den här länken och trycker på knappen:\n\n{link}\n\nLänken fungerar i {days} dagar. Om du inte har begärt detta kan du bortse från meddelandet, så skickas ingenting till dig.\n",
        form_received: "Om adressen är giltig är ett bekräftelsemeddelande på väg.",
        form_invalid_title: "Kontrollera formuläret",
        form_name_invalid: "Ange ett namn på högst 200 tecken, på en rad.",
        form_email_invalid: "Ange en e-postadress som kan ta emot e-post.",
        form_refused_title: "Inte tillåtet",
        form_refused: "Det här formuläret kan inte skickas från den här webbplatsen.",
        form_busy_title: "För många inskickningar",
        form_busy: "För många inskickningar. Vänta en stund och försök igen.",
        form_unavailable_title: "Inte tillgängligt",
        form_unavailable: "Formuläret kan inte skickas just nu. Försök igen senare.",
    },
    Text {
        lang: "sw",
        dir: "ltr",
        endonym: "Kiswahili",
        unsubscribe_title: "Jiondoe",
        unsubscribe_ask: "Je, ungependa kuacha kupokea barua pepe za masoko?",
        unsubscribe_button: "Jiondoe",
        unsubscribed: "Umejiondoa.",
        link_not_valid: "Kiungo hiki si halali.",
        link_title: "Kiungo si halali",
        link_expired: "Kiungo hiki si halali au muda wake umeisha.",
        went_wrong: "Kuna hitilafu iliyotokea. Tafadhali jaribu tena.",
        confirm_title: "Thibitisha",
        confirm_heading: "Thibitisha: {purpose}",
        confirm_button: "Ndiyo, nitumie hivi",
        confirmed_title: "Imethibitishwa",
        confirmed: "Asante. Utapokea: {purpose}.",
        preferences_title: "Mapendeleo yako",
        preferences_heading: "Unachopenda kupokea",
        stop: "Acha",
        start: "Anza",
        receiving: "Unapokea hivi.",
        pending: "Inasubiri uthibitisho wako.",
        not_receiving: "Hupokei hivi.",
        not_available: "Chaguo hilo halipatikani.",
        other_languages: "Lugha nyingine",
        purposes: [
            "Ujumbe wa masoko",
            "Jarida",
            "Mialiko ya matukio",
            "Habari za bidhaa",
            "Ujumbe",
        ],
        mail_subject: "Tafadhali thibitisha usajili wako",
        mail_body: "Habari {name},\n\nMtu ameomba anwani hii ipokee: {purpose}.\nIli kuthibitisha, fungua kiungo hiki na ubonyeze kitufe:\n\n{link}\n\nKiungo kinafanya kazi kwa siku {days}. Kama hukuomba hili, puuza ujumbe huu na hakuna kitakachotumwa kwako.\n",
        form_received: "Ikiwa anwani ni halali, ujumbe wa uthibitisho uko njiani.",
        form_invalid_title: "Kagua fomu",
        form_name_invalid: "Tafadhali weka jina lisilozidi herufi 200, kwenye mstari mmoja.",
        form_email_invalid: "Tafadhali weka anwani ya barua pepe inayoweza kupokea barua.",
        form_refused_title: "Hairuhusiwi",
        form_refused: "Fomu hii haiwezi kutumwa kutoka tovuti hii.",
        form_busy_title: "Maombi mengi mno",
        form_busy: "Maombi ni mengi mno. Tafadhali subiri kidogo kisha ujaribu tena.",
        form_unavailable_title: "Haipatikani",
        form_unavailable: "Fomu haiwezi kutumwa kwa sasa. Tafadhali jaribu tena baadaye.",
    },
    Text {
        lang: "ur",
        dir: "rtl",
        endonym: "اردو",
        unsubscribe_title: "رکنیت ختم کریں",
        unsubscribe_ask: "کیا آپ مارکیٹنگ ای میل موصول کرنا بند کرنا چاہتے ہیں؟",
        unsubscribe_button: "رکنیت ختم کریں",
        unsubscribed: "آپ کی رکنیت ختم ہو گئی ہے۔",
        link_not_valid: "یہ لنک درست نہیں ہے۔",
        link_title: "لنک درست نہیں",
        link_expired: "یہ لنک درست نہیں ہے یا اس کی مدت ختم ہو چکی ہے۔",
        went_wrong: "کچھ غلط ہو گیا۔ براہِ کرم دوبارہ کوشش کریں۔",
        confirm_title: "تصدیق کریں",
        confirm_heading: "تصدیق کریں: {purpose}",
        confirm_button: "جی ہاں، یہ مجھے بھیجیں",
        confirmed_title: "تصدیق ہو گئی",
        confirmed: "شکریہ۔ آپ کو موصول ہوگا: {purpose}۔",
        preferences_title: "آپ کی ترجیحات",
        preferences_heading: "آپ کیا موصول کرنا چاہیں گے",
        stop: "روکیں",
        start: "شروع کریں",
        receiving: "آپ یہ موصول کرتے ہیں۔",
        pending: "آپ کی تصدیق کا انتظار ہے۔",
        not_receiving: "آپ یہ موصول نہیں کرتے۔",
        not_available: "یہ انتخاب دستیاب نہیں ہے۔",
        other_languages: "دیگر زبانیں",
        purposes: [
            "مارکیٹنگ پیغامات",
            "نیوز لیٹر",
            "تقریبات کے دعوت نامے",
            "مصنوعات کی تازہ کاریاں",
            "پیغامات",
        ],
        mail_subject: "براہِ کرم اپنی رکنیت کی تصدیق کریں",
        mail_body: "محترم {name}،\n\nکسی نے درخواست کی ہے کہ اس پتے پر یہ موصول ہو: {purpose}۔\nتصدیق کے لیے یہ لنک کھولیں اور بٹن دبائیں:\n\n{link}\n\nیہ لنک {days} دن تک کارآمد ہے۔ اگر آپ نے یہ درخواست نہیں کی تو اس پیغام کو نظر انداز کر دیں، آپ کو کچھ نہیں بھیجا جائے گا۔\n",
        form_received: "اگر پتہ درست ہے تو تصدیقی پیغام روانہ ہو چکا ہے۔",
        form_invalid_title: "فارم کی جانچ کریں",
        form_name_invalid: "براہِ کرم ایک سطر میں زیادہ سے زیادہ 200 حروف کا نام درج کریں۔",
        form_email_invalid: "براہِ کرم ایسا ای میل پتہ درج کریں جو پیغامات موصول کر سکے۔",
        form_refused_title: "اجازت نہیں",
        form_refused: "یہ فارم اس ویب سائٹ سے نہیں بھیجا جا سکتا۔",
        form_busy_title: "بہت زیادہ درخواستیں",
        form_busy: "بہت زیادہ درخواستیں موصول ہوئی ہیں۔ براہِ کرم ذرا انتظار کریں اور دوبارہ کوشش کریں۔",
        form_unavailable_title: "دستیاب نہیں",
        form_unavailable: "فارم اس وقت نہیں بھیجا جا سکتا۔ براہِ کرم بعد میں دوبارہ کوشش کریں۔",
    },
    Text {
        lang: "vi",
        dir: "ltr",
        endonym: "Tiếng Việt",
        unsubscribe_title: "Hủy đăng ký",
        unsubscribe_ask: "Ngừng nhận email tiếp thị?",
        unsubscribe_button: "Hủy đăng ký",
        unsubscribed: "Quý vị đã hủy đăng ký.",
        link_not_valid: "Liên kết này không hợp lệ.",
        link_title: "Liên kết không hợp lệ",
        link_expired: "Liên kết này không hợp lệ hoặc đã hết hạn.",
        went_wrong: "Đã xảy ra lỗi. Vui lòng thử lại.",
        confirm_title: "Xác nhận",
        confirm_heading: "Xác nhận: {purpose}",
        confirm_button: "Có, hãy gửi cho tôi",
        confirmed_title: "Đã xác nhận",
        confirmed: "Xin cảm ơn. Quý vị sẽ nhận được: {purpose}.",
        preferences_title: "Tùy chọn của quý vị",
        preferences_heading: "Nội dung quý vị muốn nhận",
        stop: "Dừng",
        start: "Bắt đầu",
        receiving: "Quý vị đang nhận các nội dung này.",
        pending: "Đang chờ quý vị xác nhận.",
        not_receiving: "Quý vị không nhận các nội dung này.",
        not_available: "Lựa chọn đó không có sẵn.",
        other_languages: "Ngôn ngữ khác",
        purposes: [
            "Tin nhắn tiếp thị",
            "Bản tin",
            "Lời mời tham dự sự kiện",
            "Cập nhật sản phẩm",
            "Tin nhắn",
        ],
        mail_subject: "Vui lòng xác nhận đăng ký của quý vị",
        mail_body: "Xin chào {name},\n\nCó người đã yêu cầu địa chỉ này nhận: {purpose}.\nĐể xác nhận, hãy mở liên kết này và nhấn nút:\n\n{link}\n\nLiên kết có hiệu lực trong {days} ngày. Nếu quý vị không yêu cầu việc này, hãy bỏ qua thư này và sẽ không có gì được gửi cho quý vị.\n",
        form_received: "Nếu địa chỉ hợp lệ, một thư xác nhận đang được gửi đi.",
        form_invalid_title: "Kiểm tra biểu mẫu",
        form_name_invalid: "Vui lòng nhập tên tối đa 200 ký tự, trên một dòng.",
        form_email_invalid: "Vui lòng nhập địa chỉ email có thể nhận thư.",
        form_refused_title: "Không được phép",
        form_refused: "Không thể gửi biểu mẫu này từ trang web này.",
        form_busy_title: "Quá nhiều lượt gửi",
        form_busy: "Quá nhiều lượt gửi. Vui lòng đợi một lát rồi thử lại.",
        form_unavailable_title: "Không khả dụng",
        form_unavailable: "Hiện chưa thể gửi biểu mẫu. Vui lòng thử lại sau.",
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
        form_invalid_title: "请检查表单",
        form_name_invalid: "请输入不超过 200 个字符的姓名，且不要换行。",
        form_email_invalid: "请输入能够接收邮件的电子邮件地址。",
        form_refused_title: "不允许",
        form_refused: "无法从此网站发送此表单。",
        form_busy_title: "提交次数过多",
        form_busy: "提交次数过多。请稍候再试。",
        form_unavailable_title: "暂不可用",
        form_unavailable: "目前无法发送表单。请稍后再试。",
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
        // An empty range and `*` name no page, so `text_for` passes over them; q=0 refuses one.
        if weight > 0 {
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

    // CRM-T259: a range with q=0, `*`, or nothing chooses nothing, and a q value is 0 to 1 with up
    // to three decimals.
    #[test]
    fn a_zero_star_or_empty_range_chooses_nothing() {
        assert_eq!(negotiate(None, Some("de;q=0, fr;q=0.1")).lang, "fr");
        assert_eq!(negotiate(None, Some("*, de;q=0.5")).lang, "de");
        assert_eq!(negotiate(None, Some(", de;q=0.5")).lang, "de");
        assert_eq!(
            negotiate(None, Some("de;q=0")).lang,
            "en",
            "q=0 refuses it, so English"
        );
        assert_eq!(negotiate(None, Some("*")).lang, "en");
        assert_eq!(negotiate(None, Some("")).lang, "en");
    }

    #[test]
    fn a_quality_is_zero_to_one_in_thousandths() {
        assert_eq!(quality("1"), 1000);
        assert_eq!(quality("1.000"), 1000);
        assert_eq!(quality("1.001"), 0);
        assert_eq!(quality("0.5"), 500);
        assert_eq!(quality("0.123"), 123);
        assert_eq!(quality("0.1234"), 0);
        assert_eq!(quality("0.1x"), 0);
        assert_eq!(quality("2"), 0);
    }

    #[test]
    fn the_browser_s_language_is_chosen_by_weight_then_order() {
        assert_eq!(negotiate(None, Some("de-AT,de;q=0.9,en;q=0.5")).lang, "de");
        assert_eq!(
            negotiate(None, Some("tr, zh-CN;q=0.8, en;q=0.7")).lang,
            "zh"
        );
        assert_eq!(negotiate(None, Some("en;q=0.2, ar;q=0.9")).lang, "ar");
        assert_eq!(negotiate(None, Some("cy, fr")).lang, "cy");
        assert_eq!(negotiate(None, Some("fr;q=0, hi")).lang, "hi");
        assert_eq!(
            negotiate(None, Some("tr, pl")).lang,
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
            negotiate(Some("tr-TR"), Some("de")).lang,
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
                text.form_invalid_title,
                text.form_name_invalid,
                text.form_email_invalid,
                text.form_refused_title,
                text.form_refused,
                text.form_busy_title,
                text.form_busy,
                text.form_unavailable_title,
                text.form_unavailable,
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
                for (theirs, ours) in [
                    (text.form_invalid_title, english.form_invalid_title),
                    (text.form_name_invalid, english.form_name_invalid),
                    (text.form_email_invalid, english.form_email_invalid),
                    (text.form_refused_title, english.form_refused_title),
                    (text.form_refused, english.form_refused),
                    (text.form_busy_title, english.form_busy_title),
                    (text.form_busy, english.form_busy),
                    (text.form_unavailable_title, english.form_unavailable_title),
                    (text.form_unavailable, english.form_unavailable),
                ] {
                    assert_ne!(
                        theirs, ours,
                        "{}: the form's refusals (CRM-T182)",
                        text.lang
                    );
                }
            }
            assert_eq!(text.dir == "rtl", matches!(text.lang, "ar" | "ur"));
        }
        assert_eq!(TEXTS.len(), 21, "the UI's 21 languages");
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
