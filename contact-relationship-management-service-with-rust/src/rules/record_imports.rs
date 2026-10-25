//! Contact and account import (CRM-R43): the mappings, and the judging of each row.
//!
//! Pure and DB-free. The reading of the CSV is the lead import's ([`super::imports::parse`]); what
//! is different here is what a row must hold. Contacts and accounts are references to the person
//! and organization services, so the controller looks each one up there before saving.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::forms::{valid_email, valid_name};
use super::tokens;

/// Contacts saved per transaction (each needs a lookup first, so batches are smaller than leads').
pub const BATCH: usize = 50;
/// Longest cell kept for a short text field.
const MAX_SHORT: usize = 200;

/// Which column holds what for a contact import. Each value is a header from the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ContactImportMapping")]
#[serde(deny_unknown_fields)]
pub struct ContactMapping {
    /// The column with the contact's name (required).
    #[serde(default)]
    pub display_name: Option<String>,
    /// The column with the address (required): it goes to the person service, never into the CRM.
    #[serde(default)]
    pub email: Option<String>,
    /// The column with the given name.
    #[serde(default)]
    pub given_name: Option<String>,
    /// The column with the family name.
    #[serde(default)]
    pub family_name: Option<String>,
    /// The column with the job title.
    #[serde(default)]
    pub job_title: Option<String>,
}

/// Which column holds what for an account import.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "AccountImportMapping")]
#[serde(deny_unknown_fields)]
pub struct AccountMapping {
    /// The column with the organization's name (required).
    #[serde(default)]
    pub display_name: Option<String>,
    /// A column holding an `organization:` reference; without one the organization service is
    /// asked to find or make the organization by name.
    #[serde(default)]
    pub organization_ref: Option<String>,
    /// The column with the tier (`prospect`, `customer`, `partner`, `former`); default `prospect`.
    #[serde(default)]
    pub tier: Option<String>,
    /// The column with the industry.
    #[serde(default)]
    pub industry: Option<String>,
}

/// The index of each mapped field's column.
pub type Columns = BTreeMap<&'static str, usize>;

fn find(headers: &[String], names: &[&str]) -> Option<String> {
    headers
        .iter()
        .find(|h| {
            let h = h.to_lowercase().replace(['_', '-'], " ");
            names.contains(&h.trim())
        })
        .cloned()
}

/// A contact mapping guessed from the header names.
#[must_use]
pub fn suggest_contact(headers: &[String]) -> ContactMapping {
    ContactMapping {
        display_name: find(headers, &["name", "full name", "display name", "contact"]),
        email: find(headers, &["email", "e mail", "email address", "mail"]),
        given_name: find(headers, &["first name", "given name", "forename"]),
        family_name: find(headers, &["last name", "family name", "surname"]),
        job_title: find(headers, &["title", "job title", "position", "role"]),
    }
}

/// An account mapping guessed from the header names.
#[must_use]
pub fn suggest_account(headers: &[String]) -> AccountMapping {
    AccountMapping {
        display_name: find(
            headers,
            &["name", "company", "organization", "organisation", "account"],
        ),
        organization_ref: find(
            headers,
            &["organization ref", "organisation ref", "reference"],
        ),
        tier: find(headers, &["tier", "type"]),
        industry: find(headers, &["industry", "sector"]),
    }
}

/// The column index of each mapped field.
///
/// # Errors
///
/// A message when a required field is not mapped, or a named column is not in the file.
pub fn resolve(
    fields: &[(&'static str, Option<&String>, bool)],
    headers: &[String],
) -> Result<Columns, String> {
    let mut out = BTreeMap::new();
    for (field, column, required) in fields {
        let Some(column) = column else {
            if *required {
                return Err(format!("{field} must be mapped to a column"));
            }
            continue;
        };
        let index = headers
            .iter()
            .position(|h| h == column.trim())
            .ok_or_else(|| format!("{field}: the file has no column named {column:?}"))?;
        out.insert(*field, index);
    }
    Ok(out)
}

/// The columns of a contact mapping.
///
/// # Errors
///
/// See [`resolve`].
pub fn resolve_contact(mapping: &ContactMapping, headers: &[String]) -> Result<Columns, String> {
    resolve(
        &[
            ("display_name", mapping.display_name.as_ref(), true),
            ("email", mapping.email.as_ref(), true),
            ("given_name", mapping.given_name.as_ref(), false),
            ("family_name", mapping.family_name.as_ref(), false),
            ("job_title", mapping.job_title.as_ref(), false),
        ],
        headers,
    )
}

/// The columns of an account mapping.
///
/// # Errors
///
/// See [`resolve`].
pub fn resolve_account(mapping: &AccountMapping, headers: &[String]) -> Result<Columns, String> {
    resolve(
        &[
            ("display_name", mapping.display_name.as_ref(), true),
            ("organization_ref", mapping.organization_ref.as_ref(), false),
            ("tier", mapping.tier.as_ref(), false),
            ("industry", mapping.industry.as_ref(), false),
        ],
        headers,
    )
}

/// A contact row, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRow {
    /// The name, trimmed.
    pub name: String,
    /// The address; it is sent to the person service and kept nowhere here.
    pub email: String,
    /// The given name, when there is one.
    pub given_name: Option<String>,
    /// The family name, when there is one.
    pub family_name: Option<String>,
    /// The job title, when there is one.
    pub job_title: Option<String>,
}

/// An account row, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    /// The name, trimmed.
    pub name: String,
    /// The organization reference, when the file gave one.
    pub organization_ref: Option<String>,
    /// The tier (lower case; `prospect` when the file gave none).
    pub tier: String,
    /// The industry, when there is one.
    pub industry: Option<String>,
}

fn cell(cells: &[String], columns: &Columns, field: &str) -> Option<String> {
    columns
        .get(field)
        .and_then(|i| cells.get(*i))
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
}

fn short(value: Option<String>, what: &str) -> Result<Option<String>, String> {
    match value {
        Some(v) if v.chars().count() > MAX_SHORT || v.chars().any(char::is_control) => {
            Err(format!("the {what} is too long or has a line break"))
        }
        other => Ok(other),
    }
}

fn width_check(cells: &[String], width: usize) -> Result<(), String> {
    if cells.len() == width {
        Ok(())
    } else {
        Err(format!(
            "has {} columns, the header has {width}",
            cells.len()
        ))
    }
}

/// Judge one contact row.
///
/// # Errors
///
/// The reason the row cannot be imported.
pub fn judge_contact(
    cells: &[String],
    columns: &Columns,
    width: usize,
) -> Result<ContactRow, String> {
    width_check(cells, width)?;
    let name = cell(cells, columns, "display_name").ok_or("has no name")?;
    if !valid_name(&name) {
        return Err("the name is too long or has a line break".to_string());
    }
    let email = cell(cells, columns, "email").ok_or("has no email address")?;
    if !valid_email(&email) {
        return Err("the email address is not usable".to_string());
    }
    Ok(ContactRow {
        name,
        email,
        given_name: short(cell(cells, columns, "given_name"), "given name")?,
        family_name: short(cell(cells, columns, "family_name"), "family name")?,
        job_title: short(cell(cells, columns, "job_title"), "job title")?,
    })
}

/// Judge one account row.
///
/// # Errors
///
/// The reason the row cannot be imported.
pub fn judge_account(
    cells: &[String],
    columns: &Columns,
    width: usize,
) -> Result<AccountRow, String> {
    width_check(cells, width)?;
    let name = cell(cells, columns, "display_name").ok_or("has no name")?;
    if !valid_name(&name) {
        return Err("the name is too long or has a line break".to_string());
    }
    let organization_ref = match cell(cells, columns, "organization_ref") {
        Some(urn) => {
            let ok = entity_ref::EntityRef::from_str(&urn)
                .is_ok_and(|r| r.entity_type == entity_ref::EntityType::Organization);
            if !ok {
                return Err("the organization reference is not an organization: URN".to_string());
            }
            Some(urn)
        }
        None => None,
    };
    let tier = match cell(cells, columns, "tier") {
        Some(t) if tokens::is_token(tokens::ACCOUNT_TIERS, &t.to_lowercase()) => t.to_lowercase(),
        Some(_) => {
            return Err(format!(
                "the tier is not one of {}",
                tokens::ACCOUNT_TIERS.join(", ")
            ));
        }
        None => "prospect".to_string(),
    };
    Ok(AccountRow {
        name,
        organization_ref,
        tier,
        industry: short(cell(cells, columns, "industry"), "industry")?,
    })
}

/// A name as compared between accounts: lower case, single spaces.
#[must_use]
pub fn account_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn cells(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn mappings_are_suggested_from_the_headers() {
        let h = headers(&[
            "Full Name",
            "E-mail",
            "First name",
            "Surname",
            "Job Title",
            "Notes",
        ]);
        let m = suggest_contact(&h);
        assert_eq!(m.display_name.as_deref(), Some("Full Name"));
        assert_eq!(m.email.as_deref(), Some("E-mail"));
        assert_eq!(m.given_name.as_deref(), Some("First name"));
        assert_eq!(m.family_name.as_deref(), Some("Surname"));
        assert_eq!(m.job_title.as_deref(), Some("Job Title"));
        let a = suggest_account(&headers(&["Company", "Sector", "Tier"]));
        assert_eq!(a.display_name.as_deref(), Some("Company"));
        assert_eq!(a.industry.as_deref(), Some("Sector"));
        assert_eq!(a.tier.as_deref(), Some("Tier"));
    }

    #[test]
    fn a_contact_needs_a_name_and_an_address() {
        let h = headers(&["Name", "Email", "Title"]);
        let columns = resolve_contact(&suggest_contact(&h), &h).unwrap();
        let ok = judge_contact(&cells(&[" Ada ", "ada@x.test", "Analyst"]), &columns, 3).unwrap();
        assert_eq!(ok.name, "Ada");
        assert_eq!(ok.job_title.as_deref(), Some("Analyst"));
        assert!(
            judge_contact(&cells(&["", "a@x.test", ""]), &columns, 3)
                .unwrap_err()
                .contains("no name")
        );
        assert!(
            judge_contact(&cells(&["Ada", "", ""]), &columns, 3)
                .unwrap_err()
                .contains("no email")
        );
        assert!(
            judge_contact(&cells(&["Ada", "nope", ""]), &columns, 3)
                .unwrap_err()
                .contains("email")
        );
        assert!(judge_contact(&cells(&["Ada\nBcc: x", "a@x.test", ""]), &columns, 3).is_err());
        assert!(
            judge_contact(&cells(&["Ada", "a@x.test"]), &columns, 3)
                .unwrap_err()
                .contains("2 columns")
        );
        assert!(resolve_contact(&ContactMapping::default(), &h).is_err());
        let no_email = ContactMapping {
            display_name: Some("Name".into()),
            ..ContactMapping::default()
        };
        assert!(
            resolve_contact(&no_email, &h)
                .unwrap_err()
                .contains("email")
        );
    }

    #[test]
    fn an_account_is_judged() {
        let h = headers(&["Company", "Ref", "Tier", "Industry"]);
        let mapping = AccountMapping {
            display_name: Some("Company".into()),
            organization_ref: Some("Ref".into()),
            tier: Some("Tier".into()),
            industry: Some("Industry".into()),
        };
        let columns = resolve_account(&mapping, &h).unwrap();
        let row = |v: &[&str]| judge_account(&cells(v), &columns, 4);
        let ok = row(&["Engine Co", "", "Customer", "Machines"]).unwrap();
        assert_eq!(ok.tier, "customer");
        assert_eq!(ok.organization_ref, None);
        let urn = format!("organization:{}", uuid::Uuid::new_v4());
        assert_eq!(
            row(&["Engine Co", &urn, "", ""]).unwrap().organization_ref,
            Some(urn)
        );
        assert_eq!(row(&["Engine Co", "", "", ""]).unwrap().tier, "prospect");
        assert!(row(&["Engine Co", "person:123", "", ""]).is_err());
        assert!(
            row(&["Engine Co", "", "vip", ""])
                .unwrap_err()
                .contains("tier")
        );
        assert!(row(&["", "", "", ""]).unwrap_err().contains("no name"));
    }

    #[test]
    fn account_names_compare_loosely() {
        assert_eq!(account_key("  Engine   Co "), account_key("engine co"));
        assert_ne!(account_key("Engine Co"), account_key("Engine Corp"));
    }
}
