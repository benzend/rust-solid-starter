//! The one place that knows about an error-tracking service.
//!
//! Everything else reports errors by logging them: `tracing::error!(...)`.
//! Error trackers consume `tracing` events as a subscriber layer, so plugging
//! one in means filling in `init` below; no handler code changes. See "Adding
//! error tracking" in the README for a worked Sentry example.

use std::any::Any;

use tracing::Subscriber;
use tracing_subscriber::{Layer, layer::Identity, registry::LookupSpan};

/// Keeps the reporter's client alive until shutdown so it can flush pending
/// events. Hold it in `main` for the life of the program.
pub struct Guard {
    _client: Option<Box<dyn Any + Send>>,
}

/// Returns the layer that forwards `tracing` events to an error tracker, or
/// `None` when reporting is off. Today it's always off.
pub fn init<S>() -> (Option<impl Layer<S> + Send + Sync>, Guard)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    (None::<Identity>, Guard { _client: None })
}
