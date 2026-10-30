//! The **pure core** (CRM-D3–D5): lifecycle machines, deterministic
//! lead scoring with breakdown, KPI arithmetic (forecast / ROI / CLV
//! / win rate), SLA deadline + breach derivation, and segment
//! evaluation with the structural consent gate — DB-free and
//! exhaustively unit-tested. Controllers wire these; they never
//! re-implement them.

pub mod analytics;
pub mod attachments;
pub mod attribution;
pub mod audit_chain;
pub mod calendar;
pub mod custom_fields;
pub mod engagement;
pub mod external_links;
pub mod forms;
pub mod fx;
pub mod imports;
pub mod lifecycle;
pub mod merge;
pub mod privacy;
pub mod quota;
pub mod record_imports;
pub mod reports;
pub mod scoring;
pub mod search;
pub mod segment;
pub mod sla;
pub mod suggest;
pub mod tags;
pub mod templates;
pub mod tokens;
pub mod views;
pub mod webhooks;
pub mod workflows;
pub mod wpm_events;
