//! The record the matcher compares.

use serde::{Deserialize, Serialize};

/// A contact, reduced to the fields that help tell people apart.
///
/// Every field is optional. A field that is empty on either side of a
/// comparison is *missing*: it adds no evidence for or against a match.
///
/// The matcher never stores a contact. In the CRM the service holds no email
/// addresses itself (they live on the person record), so a caller fetches
/// them at comparison time and drops them afterwards.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Contact {
    /// The person record this contact wraps (`person_ref`). Two contacts that
    /// share one are the same person, with certainty.
    pub person_ref: Option<String>,
    /// Given name. Only the first word is compared; middle names are ignored.
    pub given_name: Option<String>,
    /// Family name.
    pub family_name: Option<String>,
    /// Display name, used to fill in a given or family name that is missing:
    /// `Dr Ana Ruiz`, `Ruiz, Ana`.
    pub display_name: Option<String>,
    /// Email addresses. A person often has several.
    pub emails: Vec<String>,
    /// Phone numbers, in any common format.
    pub phones: Vec<String>,
    /// Employer or account name.
    pub organization: Option<String>,
    /// Job title.
    pub job_title: Option<String>,
}

impl Contact {
    /// An empty contact; set the fields you have.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the person reference.
    #[must_use]
    pub fn person_ref(mut self, value: impl Into<String>) -> Self {
        self.person_ref = Some(value.into());
        self
    }

    /// Set the given name.
    #[must_use]
    pub fn given_name(mut self, value: impl Into<String>) -> Self {
        self.given_name = Some(value.into());
        self
    }

    /// Set the family name.
    #[must_use]
    pub fn family_name(mut self, value: impl Into<String>) -> Self {
        self.family_name = Some(value.into());
        self
    }

    /// Set the display name.
    #[must_use]
    pub fn display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = Some(value.into());
        self
    }

    /// Add an email address.
    #[must_use]
    pub fn email(mut self, value: impl Into<String>) -> Self {
        self.emails.push(value.into());
        self
    }

    /// Add a phone number.
    #[must_use]
    pub fn phone(mut self, value: impl Into<String>) -> Self {
        self.phones.push(value.into());
        self
    }

    /// Set the organisation.
    #[must_use]
    pub fn organization(mut self, value: impl Into<String>) -> Self {
        self.organization = Some(value.into());
        self
    }

    /// Set the job title.
    #[must_use]
    pub fn job_title(mut self, value: impl Into<String>) -> Self {
        self.job_title = Some(value.into());
        self
    }
}
