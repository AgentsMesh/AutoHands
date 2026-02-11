//! Webhook interface module.
//!
//! Provides event-driven trigger capabilities via HTTP webhooks.
//! All webhook events are converted to RunLoop events for unified processing.

mod handler;
pub mod registry;
mod types;

pub use handler::{
    delete_webhook, get_webhook, handle_github_webhook, handle_webhook, list_webhooks,
    register_webhook,
};
pub use registry::WebhookRegistry;
pub use types::{WebhookEvent, WebhookRegistration, WebhookResponse};
