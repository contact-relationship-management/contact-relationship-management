//! The header names an import recognises for each field, by language (CRM-T169, CRM-R95).
//!
//! An import guesses its mapping from the file's header row (CRM-R34, CRM-R43); the mapping can
//! always be set by hand. The names are listed in the UI's 21 languages (`ar`, `bn`, `cy`, `da`, `de`, `en`, `es`, `fr`, `hi`, `id`, `it`, `ja`, `ko`, `nl`, `pt`, `ru`, `sv`, `sw`, `ur`, `vi`, `zh`),
//! so a spreadsheet exported in any of them is mapped as an English one
//! is. All but English were drafted by the AI assistant and not checked by a native speaker.
//! Headers are compared in lower case, with `_` and `-` read as spaces, runs of spaces as one, and
//! the accents of Latin letters ignored (`Prénom` and `prenom` are the same header).

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// A field an import can map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The full name.
    Name,
    /// An email address.
    Email,
    /// The given name.
    GivenName,
    /// The family name.
    FamilyName,
    /// The job title.
    JobTitle,
    /// The organization a person works for, or an account's own name.
    Company,
    /// An organization-service reference.
    OrganizationRef,
    /// An account tier.
    Tier,
    /// An account's industry.
    Industry,
}

type Names = &'static [(&'static str, &'static [&'static str])];

/// The names of [`Field::Name`], by language.
const NAME: Names = &[
    ("en", &["name", "full name", "display name", "contact"]),
    ("ar", &["الاسم", "الاسم الكامل", "جهة الاتصال"]),
    ("cy", &["enw", "enw llawn", "cyswllt"]),
    ("de", &["vollständiger name", "kontakt", "anzeigename"]),
    ("es", &["nombre", "nombre completo", "contacto"]),
    ("fr", &["nom", "nom complet", "contact"]),
    ("hi", &["नाम", "पूरा नाम", "संपर्क"]),
    ("zh", &["姓名", "全名", "名称", "联系人"]),
    ("bn", &["নাম", "পুরো নাম", "যোগাযোগ"]),
    ("da", &["navn", "fulde navn", "kontakt"]),
    ("id", &["nama", "nama lengkap", "kontak"]),
    ("it", &["nome completo", "nominativo", "contatto"]),
    ("ja", &["氏名", "名前", "連絡先"]),
    ("ko", &["성명", "전체 이름", "연락처"]),
    ("nl", &["naam", "volledige naam", "weergavenaam"]),
    ("pt", &["nome", "nome completo", "contato"]),
    ("ru", &["фио", "полное имя", "контакт", "имя и фамилия"]),
    (
        "sv",
        &["namn", "fullständigt namn", "visningsnamn", "kontakt"],
    ),
    ("sw", &["jina", "jina kamili", "mawasiliano"]),
    ("ur", &["نام", "پورا نام", "رابطہ"]),
    ("vi", &["họ và tên", "họ tên", "tên đầy đủ", "liên hệ"]),
];

/// The names of [`Field::Email`], by language.
const EMAIL: Names = &[
    ("en", &["email", "e mail", "email address", "mail"]),
    ("ar", &["البريد الإلكتروني", "بريد إلكتروني"]),
    ("cy", &["e bost", "ebost", "cyfeiriad e bost"]),
    ("de", &["e mail adresse", "email adresse"]),
    ("es", &["correo", "correo electrónico"]),
    (
        "fr",
        &["courriel", "adresse e mail", "adresse électronique"],
    ),
    ("hi", &["ईमेल", "ई मेल"]),
    ("zh", &["电子邮件", "邮箱", "电子邮箱"]),
    ("bn", &["ইমেইল", "ইমেল", "ই মেইল"]),
    ("da", &["e mail", "email", "mailadresse"]),
    ("id", &["email", "surel", "alamat email"]),
    (
        "it",
        &[
            "email",
            "indirizzo email",
            "indirizzo e-mail",
            "posta elettronica",
        ],
    ),
    ("ja", &["メールアドレス", "メール", "電子メール"]),
    ("ko", &["이메일", "이메일 주소", "전자우편"]),
    ("nl", &["e-mail", "e-mailadres", "e-mail adres"]),
    ("pt", &["e-mail", "email", "endereço de e-mail"]),
    (
        "ru",
        &[
            "электронная почта",
            "почта",
            "e-mail",
            "адрес электронной почты",
        ],
    ),
    ("sv", &["e-post", "epost", "e-postadress", "mejladress"]),
    ("sw", &["barua pepe", "anwani ya barua pepe"]),
    ("ur", &["ای میل", "ای میل ایڈریس", "برقی ڈاک"]),
    ("vi", &["email", "thư điện tử", "địa chỉ email"]),
];

/// The names of [`Field::GivenName`], by language.
const GIVEN_NAME: Names = &[
    ("en", &["first name", "given name", "forename"]),
    ("ar", &["الاسم الأول"]),
    ("cy", &["enw cyntaf"]),
    ("de", &["vorname"]),
    ("es", &["nombre de pila"]),
    ("fr", &["prénom"]),
    ("hi", &["पहला नाम"]),
    ("zh", &["名"]),
    ("bn", &["প্রথম নাম"]),
    ("da", &["fornavn"]),
    ("id", &["nama depan"]),
    ("it", &["nome di battesimo", "primo nome"]),
    ("ja", &["名"]),
    ("ko", &["이름"]),
    ("nl", &["voornaam", "roepnaam"]),
    ("pt", &["primeiro nome", "nome próprio"]),
    ("ru", &["имя"]),
    ("sv", &["förnamn"]),
    ("sw", &["jina la kwanza"]),
    ("ur", &["پہلا نام"]),
    ("vi", &["tên riêng", "tên gọi"]),
];

/// The names of [`Field::FamilyName`], by language.
const FAMILY_NAME: Names = &[
    ("en", &["last name", "family name", "surname"]),
    ("ar", &["اسم العائلة"]),
    ("cy", &["cyfenw"]),
    ("de", &["nachname", "familienname"]),
    ("es", &["apellido", "apellidos"]),
    ("fr", &["nom de famille"]),
    ("hi", &["उपनाम"]),
    ("zh", &["姓"]),
    ("bn", &["পদবি", "শেষ নাম"]),
    ("da", &["efternavn"]),
    ("id", &["nama belakang", "nama keluarga"]),
    ("it", &["cognome"]),
    ("ja", &["姓"]),
    ("ko", &["성", "성씨"]),
    ("nl", &["achternaam", "familienaam"]),
    ("pt", &["sobrenome", "último nome", "nome de família"]),
    ("ru", &["фамилия"]),
    ("sv", &["efternamn", "släktnamn"]),
    ("sw", &["jina la ukoo", "jina la mwisho"]),
    ("ur", &["آخری نام", "خاندانی نام"]),
    ("vi", &["họ"]),
];

/// The names of [`Field::JobTitle`], by language.
const JOB_TITLE: Names = &[
    ("en", &["title", "job title", "position", "role"]),
    ("ar", &["المسمى الوظيفي", "الوظيفة"]),
    ("cy", &["teitl swydd", "swydd"]),
    ("de", &["berufsbezeichnung", "titel", "funktion"]),
    ("es", &["cargo", "puesto"]),
    ("fr", &["poste", "fonction", "titre"]),
    ("hi", &["पद"]),
    ("zh", &["职位", "职务"]),
    ("bn", &["চাকরির পদ", "পদমর্যাদা"]),
    ("da", &["stilling", "jobtitel", "titel"]),
    ("id", &["jabatan", "judul pekerjaan", "posisi"]),
    (
        "it",
        &["qualifica", "posizione", "ruolo", "titolo professionale"],
    ),
    ("ja", &["役職", "肩書", "職名"]),
    ("ko", &["직책", "직위", "직함"]),
    ("nl", &["functie", "functietitel", "beroep"]),
    ("pt", &["cargo", "função", "título"]),
    ("ru", &["должность", "позиция"]),
    ("sv", &["titel", "befattning", "roll", "yrkestitel"]),
    ("sw", &["cheo", "nafasi", "wadhifa"]),
    ("ur", &["عہدہ", "منصب", "ملازمت کا عنوان"]),
    ("vi", &["chức danh", "chức vụ", "vị trí"]),
];

/// The names of [`Field::Company`], by language.
const COMPANY: Names = &[
    (
        "en",
        &["company", "organization", "organisation", "employer"],
    ),
    ("ar", &["الشركة", "المؤسسة"]),
    ("cy", &["cwmni", "sefydliad"]),
    ("de", &["firma", "unternehmen"]),
    ("es", &["empresa", "organización", "compañía"]),
    ("fr", &["entreprise", "société"]),
    ("hi", &["कंपनी", "संगठन"]),
    ("zh", &["公司", "单位", "组织"]),
    ("bn", &["কোম্পানি", "প্রতিষ্ঠান", "সংস্থা"]),
    ("da", &["virksomhed", "firma", "organisation"]),
    ("id", &["perusahaan", "organisasi", "instansi"]),
    ("it", &["azienda", "società", "organizzazione", "ditta"]),
    ("ja", &["会社", "会社名", "組織", "勤務先"]),
    ("ko", &["회사", "회사명", "조직", "기업"]),
    ("nl", &["bedrijf", "organisatie", "werkgever"]),
    ("pt", &["empresa", "organização", "companhia"]),
    ("ru", &["компания", "организация", "работодатель"]),
    ("sv", &["företag", "organisation", "arbetsgivare"]),
    ("sw", &["kampuni", "shirika", "taasisi"]),
    ("ur", &["کمپنی", "ادارہ", "تنظیم"]),
    ("vi", &["công ty", "tổ chức", "doanh nghiệp"]),
];

/// The names of [`Field::OrganizationRef`], by language.
const ORGANIZATION_REF: Names = &[
    ("en", &["organization ref", "organisation ref", "reference"]),
    ("cy", &["cyfeirnod"]),
    ("de", &["referenz"]),
    ("es", &["referencia"]),
    ("fr", &["référence"]),
    ("zh", &["编号"]),
    ("bn", &["রেফারেন্স", "প্রতিষ্ঠানের রেফারেন্স"]),
    ("da", &["reference", "organisationsreference"]),
    ("id", &["referensi", "referensi organisasi"]),
    ("it", &["riferimento organizzazione", "riferimento"]),
    ("ja", &["組織参照", "組織コード"]),
    ("ko", &["조직 참조", "조직 코드"]),
    ("nl", &["organisatiereferentie", "referentie"]),
    ("pt", &["referência", "referência da organização"]),
    ("ru", &["ссылка на организацию", "код организации"]),
    ("sv", &["organisationsreferens", "referens"]),
    ("sw", &["kumbukumbu ya shirika", "namba ya shirika"]),
    ("ur", &["ادارے کا حوالہ", "حوالہ نمبر"]),
    ("vi", &["mã tham chiếu", "mã tổ chức"]),
];

/// The names of [`Field::Tier`], by language.
const TIER: Names = &[
    ("en", &["tier", "type"]),
    ("ar", &["الفئة"]),
    ("cy", &["haen", "math"]),
    ("de", &["stufe", "typ"]),
    ("es", &["nivel", "tipo"]),
    ("fr", &["niveau"]),
    ("hi", &["श्रेणी"]),
    ("zh", &["级别", "类型"]),
    ("bn", &["স্তর", "ধরন"]),
    ("da", &["niveau", "type"]),
    ("id", &["tingkat", "tipe", "level"]),
    ("it", &["livello", "fascia", "categoria"]),
    ("ja", &["ランク", "区分", "ティア"]),
    ("ko", &["등급", "구분"]),
    ("nl", &["niveau", "categorie", "soort"]),
    ("pt", &["nível", "categoria", "tipo"]),
    ("ru", &["уровень", "категория", "тип"]),
    ("sv", &["nivå", "kategori", "klass"]),
    ("sw", &["daraja", "ngazi", "aina"]),
    ("ur", &["درجہ", "زمرہ", "قسم"]),
    ("vi", &["hạng", "cấp độ", "loại"]),
];

/// The names of [`Field::Industry`], by language.
const INDUSTRY: Names = &[
    ("en", &["industry", "sector"]),
    ("ar", &["القطاع", "الصناعة"]),
    ("cy", &["diwydiant"]),
    ("de", &["branche", "sektor"]),
    ("es", &["industria"]),
    ("fr", &["secteur", "industrie"]),
    ("hi", &["उद्योग", "क्षेत्र"]),
    ("zh", &["行业"]),
    ("bn", &["শিল্প", "খাত"]),
    ("da", &["branche", "industri", "sektor"]),
    ("id", &["industri", "sektor", "bidang usaha"]),
    ("it", &["settore", "industria"]),
    ("ja", &["業種", "業界"]),
    ("ko", &["업종", "산업"]),
    ("nl", &["bedrijfstak", "branche", "sector"]),
    ("pt", &["setor", "indústria", "ramo", "segmento"]),
    ("ru", &["отрасль", "сфера", "индустрия", "сектор"]),
    ("sv", &["bransch", "sektor"]),
    ("sw", &["sekta", "tasnia"]),
    ("ur", &["صنعت", "شعبہ"]),
    ("vi", &["ngành", "lĩnh vực", "ngành nghề"]),
];

/// The names of `field`, by language.
#[must_use]
pub const fn names(field: Field) -> Names {
    match field {
        Field::Name => NAME,
        Field::Email => EMAIL,
        Field::GivenName => GIVEN_NAME,
        Field::FamilyName => FAMILY_NAME,
        Field::JobTitle => JOB_TITLE,
        Field::Company => COMPANY,
        Field::OrganizationRef => ORGANIZATION_REF,
        Field::Tier => TIER,
        Field::Industry => INDUSTRY,
    }
}

/// A header as it is compared: lower case, `_` and `-` as spaces, one space between words, and
/// no accents on Latin letters.
#[must_use]
pub fn header_key(header: &str) -> String {
    let mut out = String::new();
    let mut after_latin = false;
    for c in header.nfd() {
        if is_combining_mark(c) {
            if !after_latin {
                out.push(c);
            }
            continue;
        }
        after_latin = c.is_ascii_alphabetic();
        let c = if matches!(c, '_' | '-') { ' ' } else { c };
        if c.is_whitespace() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out.trim_end().nfc().collect()
}

/// The first header that is a name of one of `fields` (in any language), or one of `extra`.
#[must_use]
pub fn find(headers: &[String], fields: &[Field], extra: &[&str]) -> Option<String> {
    let wanted: Vec<String> = fields
        .iter()
        .flat_map(|f| names(*f).iter().flat_map(|(_, words)| words.iter()))
        .chain(extra.iter())
        .map(|name| header_key(name))
        .collect();
    headers
        .iter()
        .find(|h| wanted.contains(&header_key(h)))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Field; 9] = [
        Field::Name,
        Field::Email,
        Field::GivenName,
        Field::FamilyName,
        Field::JobTitle,
        Field::Company,
        Field::OrganizationRef,
        Field::Tier,
        Field::Industry,
    ];

    fn h(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn headers_are_compared_without_case_separators_or_latin_accents() {
        assert_eq!(header_key("  First_Name "), "first name");
        assert_eq!(header_key("E-Mail"), "e mail");
        assert_eq!(header_key("Prénom"), "prenom");
        assert_eq!(header_key("PRENOM"), "prenom");
        assert_eq!(
            header_key("पहला नाम"),
            "पहला नाम",
            "a Devanagari vowel sign stays"
        );
    }

    #[test]
    fn every_field_has_names_in_the_ui_s_languages_and_none_is_two_fields() {
        let mut seen: Vec<(String, Field)> = Vec::new();
        for field in ALL {
            let languages: Vec<&str> = names(field).iter().map(|(l, _)| *l).collect();
            assert!(languages.contains(&"en"), "{field:?}");
            assert!(languages.len() >= 6, "{field:?}: {languages:?}");
            for (_, words) in names(field) {
                for w in *words {
                    let key = header_key(w);
                    if let Some((_, other)) = seen.iter().find(|(k, f)| *k == key && *f != field) {
                        panic!("{w} is a name of {other:?} and {field:?}");
                    }
                    seen.push((key, field));
                }
            }
        }
    }

    #[test]
    fn a_spreadsheet_in_another_language_is_mapped() {
        let german = h(&["Vorname", "Nachname", "E-Mail-Adresse", "Firma", "Branche"]);
        assert_eq!(
            find(&german, &[Field::GivenName], &[]).as_deref(),
            Some("Vorname")
        );
        assert_eq!(
            find(&german, &[Field::FamilyName], &[]).as_deref(),
            Some("Nachname")
        );
        assert_eq!(
            find(&german, &[Field::Email], &[]).as_deref(),
            Some("E-Mail-Adresse")
        );
        assert_eq!(
            find(&german, &[Field::Company], &[]).as_deref(),
            Some("Firma")
        );
        let french = h(&["Prenom", "Nom de famille", "Courriel", "Poste"]);
        assert_eq!(
            find(&french, &[Field::GivenName], &[]).as_deref(),
            Some("Prenom")
        );
        assert_eq!(
            find(&french, &[Field::FamilyName], &[]).as_deref(),
            Some("Nom de famille")
        );
        assert_eq!(
            find(&french, &[Field::JobTitle], &[]).as_deref(),
            Some("Poste")
        );
        let chinese = h(&["姓名", "电子邮件", "公司"]);
        assert_eq!(find(&chinese, &[Field::Name], &[]).as_deref(), Some("姓名"));
        assert_eq!(
            find(&chinese, &[Field::Email], &[]).as_deref(),
            Some("电子邮件")
        );
        let arabic = h(&["الاسم", "البريد الإلكتروني"]);
        assert_eq!(
            find(&arabic, &[Field::Email], &[]).as_deref(),
            Some("البريد الإلكتروني")
        );
        let hindi = h(&["नाम", "ईमेल"]);
        assert_eq!(find(&hindi, &[Field::Name], &[]).as_deref(), Some("नाम"));
        let welsh = h(&["Enw", "E-bost"]);
        assert_eq!(
            find(&welsh, &[Field::Email], &[]).as_deref(),
            Some("E-bost")
        );
        let spanish = h(&["Nombre", "Correo electrónico", "Empresa"]);
        assert_eq!(
            find(&spanish, &[Field::Name], &[]).as_deref(),
            Some("Nombre")
        );
        assert_eq!(
            find(&h(&["Lead"]), &[Field::Name], &["lead"]).as_deref(),
            Some("Lead")
        );
        assert_eq!(find(&h(&["Colour"]), &ALL, &[]), None);
    }

    #[test]
    fn each_language_added_in_crm_t299_maps_its_own_email_and_given_name_headers() {
        for lang in [
            "bn", "da", "id", "it", "ja", "ko", "nl", "pt", "ru", "sv", "sw", "ur", "vi",
        ] {
            for field in [Field::Email, Field::GivenName] {
                let own = names(field)
                    .iter()
                    .find(|(l, _)| *l == lang)
                    .unwrap_or_else(|| panic!("{lang} has no {field:?}"))
                    .1;
                let header = own[0];
                assert_eq!(
                    find(&h(&[header]), &[field], &[]).as_deref(),
                    Some(header),
                    "{lang} {field:?}"
                );
            }
        }
    }
}
