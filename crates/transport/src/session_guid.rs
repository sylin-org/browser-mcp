// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The opaque, unguessable per-session identity minted by the adapter (ADR-0030 Decision 4).

/// An opaque, unguessable session identity minted by the adapter and presented to the service.
/// Canonical lowercase hyphenated UUIDv4 (36 chars). Secret material (ADR-0030 Decision 4:
/// "Treat the GUID as secret in logs/audit"): [`Display`](std::fmt::Display) and
/// [`Debug`](std::fmt::Debug) both render a fixed redacted placeholder rather than the raw
/// canonical string, so a `tracing::info!(guid = %guid, ...)` or `{:?}` never leaks it into a log
/// or audit sink. Use [`SessionGuid::as_str`] ONLY for the wire handshake and the routing-map key.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionGuid(String);

impl SessionGuid {
    /// Mint a fresh CSPRNG UUIDv4 (`uuid::Uuid::new_v4()`). The adapter role calls this ONCE per
    /// adapter process and reuses the same value for the process lifetime (ADR-0030 Decision 4:
    /// "Same adapter process reuses its GUID (same group); a new adapter process mints a new one").
    /// The SERVICE's own directly-served stdio session also mints one for itself (PINS.md SS9):
    /// every session gets a real GUID, closing an isolation gap an exempt lone session would
    /// otherwise leave in a later cross-session ownership map.
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().hyphenated().to_string())
    }

    /// Parse a presented string; `Some` iff it is a valid version-4 UUID in canonical (lowercase,
    /// hyphenated, unbraced) form -- the exact form a valid [`Self::mint`] output round-trips to.
    /// Any other UUID version, or a syntactically valid UUID in a non-canonical form (uppercase,
    /// braced, urn:), is refused, matching a malformed/empty presented guid the same way.
    pub fn parse(s: &str) -> Option<Self> {
        let parsed = uuid::Uuid::parse_str(s).ok()?;
        if parsed.get_version() != Some(uuid::Version::Random) {
            return None;
        }
        if parsed.hyphenated().to_string() != s {
            return None;
        }
        Some(Self(s.to_string()))
    }

    /// The raw canonical string (for the wire handshake and the routing-map key ONLY -- never a
    /// log or audit sink; see the redacted [`Display`](std::fmt::Display)/[`Debug`](std::fmt::Debug)
    /// impls below).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionGuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<redacted-session-guid>")
    }
}

impl std::fmt::Debug for SessionGuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionGuid(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_produces_a_parseable_v4_guid() {
        let g = SessionGuid::mint();
        assert!(SessionGuid::parse(g.as_str()).is_some());
    }

    #[test]
    fn parse_rejects_empty_and_malformed() {
        assert!(SessionGuid::parse("").is_none());
        assert!(SessionGuid::parse("not-a-uuid").is_none());
    }
}
