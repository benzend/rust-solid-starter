//! In-memory fixed-window rate limiting by key (an email address or an IP).
//! The counts live in this process, which fits one app server. With several
//! servers, keep them in Postgres or Redis instead.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::http::HeaderMap;

use crate::error::{AppError, AppResult};

/// No window is longer than this, so older entries can be dropped.
const MAX_WINDOW: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Default)]
pub struct RateLimiter {
    windows: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}

impl RateLimiter {
    /// Counts one attempt for `key`, and fails once there have been more than
    /// `max` in the current `window`.
    pub fn check(&self, key: String, max: u32, window: Duration) -> AppResult<()> {
        let now = Instant::now();
        let mut windows = self.windows.lock().unwrap_or_else(PoisonError::into_inner);
        if windows.len() > 10_000 {
            windows.retain(|_, (start, _)| now.duration_since(*start) < MAX_WINDOW);
        }
        let (start, count) = windows.entry(key).or_insert((now, 0));
        if now.duration_since(*start) >= window {
            *start = now;
            *count = 0;
        }
        *count += 1;
        if *count > max {
            return Err(AppError::TooManyRequests);
        }
        Ok(())
    }
}

/// The client's IP as seen by the reverse proxy: the last X-Forwarded-For entry,
/// which kamal-proxy appends. Earlier entries are whatever the client sent.
/// Without a proxy in front there's no header, and IP limits are skipped.
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    let forwarded = headers.get("x-forwarded-for")?.to_str().ok()?;
    let ip = forwarded.rsplit(',').next()?.trim();
    (!ip.is_empty()).then(|| ip.to_owned())
}
