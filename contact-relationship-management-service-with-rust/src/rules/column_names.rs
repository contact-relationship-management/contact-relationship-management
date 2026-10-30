//! The header names an import recognises for each field, by language (CRM-T169, CRM-R95).
//!
//! An import guesses its mapping from the file's header row (CRM-R34, CRM-R43); the mapping can
//! always be set by hand. The names are listed in the UI's eight languages (`ar`, `cy`, `de`, `en`,
//! `es`, `fr`, `hi`, `zh`), so a spreadsheet exported in any of them is mapped as an English one
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

/// The names of `field`, by language.
#[must_use]
pub const fn names(field: Field) -> Names {
    match field {
        Field::Name => &[
            ("en", &["name", "full name", "display name", "contact"]),
            ("ar", &["الاسم", "الاسم الكامل", "جهة الاتصال"]),
            ("cy", &["enw", "enw llawn", "cyswllt"]),
            ("de", &["vollständiger name", "kontakt", "anzeigename"]),
            ("es", &["nombre", "nombre completo", "contacto"]),
            ("fr", &["nom", "nom complet", "contact"]),
            ("hi", &["नाम", "पूरा नाम", "संपर्क"]),
            ("zh", &["姓名", "全名", "名称", "联系人"]),
        ],
        Field::Email => &[
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
        ],
        Field::GivenName => &[
            ("en", &["first name", "given name", "forename"]),
            ("ar", &["الاسم الأول"]),
            ("cy", &["enw cyntaf"]),
            ("de", &["vorname"]),
            ("es", &["nombre de pila"]),
            ("fr", &["prénom"]),
            ("hi", &["पहला नाम"]),
            ("zh", &["名"]),
        ],
        Field::FamilyName => &[
            ("en", &["last name", "family name", "surname"]),
            ("ar", &["اسم العائلة"]),
            ("cy", &["cyfenw"]),
            ("de", &["nachname", "familienname"]),
            ("es", &["apellido", "apellidos"]),
            ("fr", &["nom de famille"]),
            ("hi", &["उपनाम"]),
            ("zh", &["姓"]),
        ],
        Field::JobTitle => &[
            ("en", &["title", "job title", "position", "role"]),
            ("ar", &["المسمى الوظيفي", "الوظيفة"]),
            ("cy", &["teitl swydd", "swydd"]),
            ("de", &["berufsbezeichnung", "titel", "funktion"]),
            ("es", &["cargo", "puesto"]),
            ("fr", &["poste", "fonction", "titre"]),
            ("hi", &["पद"]),
            ("zh", &["职位", "职务"]),
        ],
        Field::Company => &[
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
        ],
        Field::OrganizationRef => &[
            ("en", &["organization ref", "organisation ref", "reference"]),
            ("cy", &["cyfeirnod"]),
            ("de", &["referenz"]),
            ("es", &["referencia"]),
            ("fr", &["référence"]),
            ("zh", &["编号"]),
        ],
        Field::Tier => &[
            ("en", &["tier", "type"]),
            ("ar", &["الفئة"]),
            ("cy", &["haen", "math"]),
            ("de", &["stufe", "typ"]),
            ("es", &["nivel", "tipo"]),
            ("fr", &["niveau"]),
            ("hi", &["श्रेणी"]),
            ("zh", &["级别", "类型"]),
        ],
        Field::Industry => &[
            ("en", &["industry", "sector"]),
            ("ar", &["القطاع", "الصناعة"]),
            ("cy", &["diwydiant"]),
            ("de", &["branche", "sektor"]),
            ("es", &["industria"]),
            ("fr", &["secteur", "industrie"]),
            ("hi", &["उद्योग", "क्षेत्र"]),
            ("zh", &["行业"]),
        ],
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
}
