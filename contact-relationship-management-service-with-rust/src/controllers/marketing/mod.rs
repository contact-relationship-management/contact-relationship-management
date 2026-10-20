//! Marketing automation (CRM-R6–R9): consent (send-path law),
//! segments with preview, campaigns with the simulated send, and
//! nurture sequences with the idempotent advance sweep.
//!
//! The send paths re-check consent **at send time** (CRM-D6); the
//! advance sweep is idempotent per (enrolment, step) (CRM-D8) — in
//! v1 it runs via `POST /api/nurture/advance` (and the seed/demo
//! flow); a periodic `bg_pg` worker is the roadmap seam.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::clients;
use crate::mail;
use crate::metrics::Metrics;
use crate::models::_entities::{
    accounts, activities, campaigns, consent_events, contacts, deals, nurture_enrollments,
    nurture_sequences, nurture_steps, segments,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::deliveries;
use crate::models::records;
use crate::rules::{analytics, lifecycle, segment as segment_rules, tokens};
use crate::streaming;
use crate::validation::Problems;

mod campaign;
mod consent;
mod nurture;
mod public;
mod segment;

pub use campaign::*;
use consent::{apply_consent, apply_consent_in, consent_history, contact_facts, record_consent};
use nurture::{advance_nurture, create_sequence, enroll, list_sequences};
use public::{email_webhook, provider_webhook, unsubscribe_page_get, unsubscribe_post};
use segment::{create_segment, evaluate_segment, list_segments, preview_segment};

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

/// The public unsubscribe routes (outside `/api`; see `auth::is_public_path`).
pub fn public_routes() -> Routes {
    Routes::new()
        .add("/unsubscribe/{token}", get(unsubscribe_page_get))
        .add("/unsubscribe/{token}", post(unsubscribe_post))
        .add("/webhooks/email", post(email_webhook))
        .add("/webhooks/email/{provider}", post(provider_webhook))
}

/// The marketing routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/consent", post(record_consent))
        .add("/contacts/{pid}/consent", get(consent_history))
        .add("/contacts/{pid}/email-suppression", get(email_suppression))
        .add(
            "/contacts/{pid}/email-suppression",
            delete(clear_email_suppression),
        )
        .add("/segments", post(create_segment))
        .add("/segments", get(list_segments))
        .add("/segments/{pid}/preview", get(preview_segment))
        .add("/campaigns", post(create_campaign))
        .add("/campaigns", get(list_campaigns))
        .add("/campaigns/{pid}/status", post(campaign_status))
        .add("/campaigns/{pid}/run", post(run_campaign))
        .add("/campaigns/{pid}/deliveries", get(campaign_deliveries))
        .add("/campaigns/{pid}/funnel", get(campaign_funnel))
        .add("/nurture-sequences", post(create_sequence))
        .add("/nurture-sequences", get(list_sequences))
        .add("/nurture-sequences/{pid}/enrollments", post(enroll))
        .add("/nurture/advance", post(advance_nurture))
}
