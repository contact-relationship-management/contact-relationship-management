//! Lead import (CRM-R34): reading the CSV, suggesting a mapping, and judging each row.
//!
//! Pure and DB-free.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::forms::{domain_of, valid_email, valid_name};

/// Most data rows in one import.
pub const MAX_ROWS: usize = 500;
/// Most bytes of CSV text in one import.
pub const MAX_BYTES: usize = 1_000_000;
/// Leads saved per transaction.
pub const BATCH: usize = 100;

/// Which column holds what. Each value is a header from the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "LeadImportMapping")]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    /// The column with the lead's name (required).
    #[serde(default)]
    pub display_name: Option<String>,
    /// The column with the address; only its domain is kept.
    #[serde(default)]
    pub email: Option<String>,
    /// The column with the company, used only to judge duplicates.
    #[serde(default)]
    pub company: Option<String>,
}

/// A CSV read into its header and data rows.
#[derive(Debug, PartialEq, Eq)]
pub struct Table {
    /// The header row, trimmed.
    pub headers: Vec<String>,
    /// The data rows, each as many cells as the row had.
    pub rows: Vec<Vec<String>>,
}

/// Read `text` as CSV with a header row.
///
/// # Errors
///
/// A message when the text is too big, has no header, has too many rows, or is not valid CSV.
pub fn parse(text: &str) -> Result<Table, String> {
    if text.len() > MAX_BYTES {
        return Err(format!("the file is over {MAX_BYTES} bytes"));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut records = reader.records();
    let headers: Vec<String> = match records.next() {
        Some(Ok(header)) => header.iter().map(|h| h.trim().to_string()).collect(),
        Some(Err(error)) => return Err(format!("not valid CSV: {error}")),
        None => return Err("the file is empty".to_string()),
    };
    if headers.iter().all(String::is_empty) {
        return Err("the first row must name the columns".to_string());
    }
    let mut rows = Vec::new();
    for record in records {
        let record = record.map_err(|error| format!("not valid CSV: {error}"))?;
        if record.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        if rows.len() == MAX_ROWS {
            return Err(format!("more than {MAX_ROWS} rows; import in pieces"));
        }
        rows.push(record.iter().map(ToString::to_string).collect());
    }
    Ok(Table { headers, rows })
}

/// A mapping guessed from the header names.
#[must_use]
pub fn suggest(headers: &[String]) -> Mapping {
    let find = |names: &[&str]| {
        headers
            .iter()
            .find(|h| {
                let h = h.to_lowercase().replace(['_', '-'], " ");
                names.contains(&h.trim())
            })
            .cloned()
    };
    Mapping {
        display_name: find(&["name", "full name", "display name", "lead", "contact"]),
        email: find(&["email", "e mail", "email address", "mail"]),
        company: find(&["company", "organization", "organisation", "employer"]),
    }
}

/// Each mapped field's column index.
///
/// # Errors
///
/// A message when `display_name` is missing, or a named column is not in the file.
pub fn resolve(
    mapping: &Mapping,
    headers: &[String],
) -> Result<BTreeMap<&'static str, usize>, String> {
    let mut out = BTreeMap::new();
    for (field, column) in [
        ("display_name", &mapping.display_name),
        ("email", &mapping.email),
        ("company", &mapping.company),
    ] {
        let Some(column) = column else {
            if field == "display_name" {
                return Err("display_name must be mapped to a column".to_string());
            }
            continue;
        };
        let index = headers
            .iter()
            .position(|h| h == column.trim())
            .ok_or_else(|| format!("{field}: the file has no column named {column:?}"))?;
        out.insert(field, index);
    }
    Ok(out)
}

/// One row, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The name, trimmed.
    pub name: String,
    /// The domain of the address, when there is one.
    pub domain: Option<String>,
    /// The company, trimmed, when there is one.
    pub company: Option<String>,
}

/// Judge one row: the lead it would make, or why not.
///
/// # Errors
///
/// The reason the row cannot be imported.
pub fn judge(
    cells: &[String],
    columns: &BTreeMap<&'static str, usize>,
    width: usize,
) -> Result<Row, String> {
    if cells.len() != width {
        return Err(format!(
            "has {} columns, the header has {width}",
            cells.len()
        ));
    }
    let cell = |field: &str| {
        columns
            .get(field)
            .and_then(|i| cells.get(*i))
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
    };
    let name = cell("display_name").ok_or_else(|| "has no name".to_string())?;
    if !valid_name(&name) {
        return Err("the name is too long or has a line break".to_string());
    }
    let domain = match cell("email") {
        Some(address) if valid_email(&address) => domain_of(&address),
        Some(_) => return Err("the email address is not usable".to_string()),
        None => None,
    };
    Ok(Row {
        name,
        domain,
        company: cell("company"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_with_quotes_a_bom_and_blank_lines_is_read() {
        let table =
            parse("\u{feff}Name, E-mail\n\"Lovelace, Ada\",ada@x.test\n\n,\nGrace,g@y.test\n")
                .unwrap();
        assert_eq!(table.headers, ["Name", "E-mail"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0][0], "Lovelace, Ada");
    }

    #[test]
    fn bad_files_are_refused_with_a_reason() {
        assert!(parse("").is_err());
        assert!(parse(",,\n").is_err());
        assert!(parse(&"x".repeat(MAX_BYTES + 1)).is_err());
        let many = format!("Name\n{}", "a\n".repeat(MAX_ROWS + 1));
        assert!(parse(&many).unwrap_err().contains("more than"));
        assert_eq!(
            parse(&format!("Name\n{}", "a\n".repeat(MAX_ROWS)))
                .unwrap()
                .rows
                .len(),
            MAX_ROWS
        );
    }

    #[test]
    fn a_mapping_is_suggested_and_resolved() {
        let headers: Vec<String> = ["Full_Name", "E-mail", "Organisation", "Notes"]
            .map(String::from)
            .to_vec();
        let mapping = suggest(&headers);
        assert_eq!(mapping.display_name.as_deref(), Some("Full_Name"));
        assert_eq!(mapping.email.as_deref(), Some("E-mail"));
        assert_eq!(mapping.company.as_deref(), Some("Organisation"));
        let columns = resolve(&mapping, &headers).unwrap();
        assert_eq!(columns["display_name"], 0);
        assert!(resolve(&Mapping::default(), &headers).is_err());
        let wrong = Mapping {
            display_name: Some("Nope".into()),
            ..Mapping::default()
        };
        assert!(resolve(&wrong, &headers).unwrap_err().contains("Nope"));
    }

    #[test]
    fn rows_are_judged() {
        let headers: Vec<String> = ["Name", "Email", "Company"].map(String::from).to_vec();
        let columns = resolve(&suggest(&headers), &headers).unwrap();
        let row = |cells: &[&str]| {
            judge(
                &cells.iter().map(ToString::to_string).collect::<Vec<_>>(),
                &columns,
                3,
            )
        };
        let ok = row(&[" Ada Lovelace ", "ADA@Engine.test", "Engine Co"]).unwrap();
        assert_eq!(ok.name, "Ada Lovelace");
        assert_eq!(ok.domain.as_deref(), Some("engine.test"));
        assert_eq!(ok.company.as_deref(), Some("Engine Co"));
        assert_eq!(row(&["Grace", "", ""]).unwrap().domain, None);
        assert!(row(&["", "a@b.co", ""]).unwrap_err().contains("no name"));
        assert!(
            row(&["Ada", "not-an-address", ""])
                .unwrap_err()
                .contains("email")
        );
        assert!(
            row(&["Ada\nBcc: x", "", ""])
                .unwrap_err()
                .contains("line break")
        );
        assert!(row(&["Ada", "a@b.co"]).unwrap_err().contains("2 columns"));
    }
}
