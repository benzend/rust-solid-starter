//! Outgoing email. To send real email, this is the one file to change: see
//! "Sending email" in the README.

use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone, Debug)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MailError(pub String);

#[derive(Clone, Default)]
pub enum Mailer {
    /// No email service configured. Debug builds log the whole message so you
    /// can follow its links locally. Release builds log only that nothing was
    /// sent, because messages contain password reset links.
    #[default]
    Log,
    /// Keeps messages in memory, for tests.
    Memory(Arc<Mutex<Vec<Email>>>),
}

impl Mailer {
    pub fn from_env() -> Self {
        Self::Log
    }

    pub async fn send(&self, email: &Email) -> Result<(), MailError> {
        match self {
            Self::Log if cfg!(debug_assertions) => tracing::info!(
                to = %email.to,
                subject = %email.subject,
                "email not sent (no mailer configured):\n{}",
                email.text
            ),
            Self::Log => {
                tracing::warn!(subject = %email.subject, "email not sent: no mailer configured")
            }
            Self::Memory(outbox) => outbox
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(email.clone()),
        }
        Ok(())
    }
}
