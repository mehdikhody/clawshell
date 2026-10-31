//! The two ways any account-keyed operation can fail, shared by every
//! operation's own error type so the command layer maps them once.

use std::io;

/// No account has the id, or the disk refused.
#[derive(Debug)]
pub enum AccountError {
    /// No clawshell account has this id.
    Unknown,
    Io(io::Error),
}

impl From<io::Error> for AccountError {
    fn from(e: io::Error) -> Self {
        AccountError::Io(e)
    }
}

/// Lets an operation's error type carry an [`AccountError`] in its `Account`
/// variant, built from an [`AccountError`] or a bare `io::Error` with `?`.
macro_rules! wrap_account_error {
    ($error:ty) => {
        impl From<$crate::core::account_error::AccountError> for $error {
            fn from(e: $crate::core::account_error::AccountError) -> Self {
                Self::Account(e)
            }
        }

        impl From<std::io::Error> for $error {
            fn from(e: std::io::Error) -> Self {
                Self::Account(e.into())
            }
        }
    };
}

pub(super) use wrap_account_error;
