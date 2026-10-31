//! `claude://` links: the only thing the sign-in router will ever forward.
//!
//! A sign-in link carries one-time codes, so nothing here prints one: the
//! type hides its contents from `Debug` and the errors say nothing about the
//! text that was refused.

use std::fmt;

const SCHEME: &str = "claude://";
/// Browsers cap URLs far below this; anything longer is not a sign-in link.
const MAX_LEN: usize = 8_192;
/// Printable ASCII that is still unsafe to hand to another program as an
/// argument. A genuine link percent-encodes all of these.
const FORBIDDEN: &str = "\"<>\\^|`";

/// A link that passed validation. It starts with `claude://`, is one token of
/// printable ASCII, and holds none of the characters in [`FORBIDDEN`].
#[derive(Clone, PartialEq, Eq)]
pub struct ClaudeLink(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Not a `claude://` link at all.
    NotClaudeLink,
    /// A `claude://` link that is empty, too long, or holds forbidden characters.
    Malformed,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkError::NotClaudeLink => f.write_str("not a claude:// link"),
            LinkError::Malformed => f.write_str("malformed claude:// link"),
        }
    }
}

impl ClaudeLink {
    pub fn parse(raw: &str) -> Result<Self, LinkError> {
        let has_scheme = raw
            .get(..SCHEME.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(SCHEME));
        if !has_scheme {
            return Err(LinkError::NotClaudeLink);
        }
        let rest = &raw[SCHEME.len()..];
        let well_formed = !rest.is_empty()
            && raw.len() <= MAX_LEN
            && raw
                .bytes()
                .all(|b| b.is_ascii_graphic() && !FORBIDDEN.as_bytes().contains(&b));
        if well_formed {
            Ok(ClaudeLink(raw.to_string()))
        } else {
            Err(LinkError::Malformed)
        }
    }

    /// The link text, for handing to Claude. Never log it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClaudeLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClaudeLink(..)")
    }
}

/// The first command-line argument that looks like a link. Used to pick a
/// link out of a launch command line; validation happens in [`ClaudeLink::parse`].
pub fn link_argument(args: &[String]) -> Option<&str> {
    args.iter().map(String::as_str).find(|arg| {
        arg.get(..SCHEME.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(SCHEME))
    })
}
