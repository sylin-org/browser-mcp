//! Domain pattern syntax and matching (browser plugin; RECONCILIATION.md section 1).
//!
//! This module owns the domain-pattern grammar of the shared format doc section 5.1: an exact
//! host (`example.com`) or a single leading `*.` wildcard (`*.example.com`). Today it
//! implements only the SYNTACTIC half ([`is_valid_pattern`]), used to validate authored
//! patterns (the `content.security.sacred_domains` governance key, and later manifest grant
//! domains). Matching SEMANTICS -- host normalization via a real URL parser, wildcard
//! matching, and the section 5.3 negative test classes (userinfo bypass, IP literals,
//! punycode, suffix stitching, redirects) -- belong to the domain matcher task, which extends
//! this same file rather than creating a new one.
//!
//! This lives in the browser plugin, not the governance core: the governance registry
//! ([`crate::governance::config`]) constrains `content.security.sacred_domains` values to
//! valid patterns, but validates them through an injected function pointer rather than naming
//! this module directly, so the core never depends on the plugin (the a7 arch-test).

/// True when `pattern` is a syntactically valid domain pattern (shared format doc 5.1): an
/// exact host (`example.com`, `127.0.0.1`) or a single leading `*.` wildcard
/// (`*.example.com`). Lowercase ASCII only; IDN domains must be authored in punycode (A-label)
/// form. IPv6-literal patterns are not accepted by this syntactic check.
pub fn is_valid_pattern(pattern: &str) -> bool {
    if pattern.is_empty() || !pattern.is_ascii() {
        return false;
    }

    let host = match pattern.strip_prefix("*.") {
        // A `*` anywhere else (bare `*`, `*.` with an empty remainder, `**.example.com` which
        // leaves `*.example.com` containing `*`, or `foo.*.com`) is invalid.
        Some(rest) if !rest.is_empty() && !rest.contains('*') => rest,
        Some(_) => return false,
        None if pattern.contains('*') => return false,
        None => pattern,
    };

    // One or more labels separated by single `.` characters: no leading dot, no trailing dot,
    // no empty label.
    if host.starts_with('.') || host.ends_with('.') {
        return false;
    }

    host.split('.').all(is_valid_label)
}

/// A single label is 1 to 63 characters, each one of `a-z`, `0-9`, or `-`, and the label
/// neither starts nor ends with `-`. Uppercase ASCII letters are invalid (patterns are
/// authored lowercase). This grammar rejects schemes, ports, paths, userinfo, and whitespace
/// by construction, and naturally accepts IPv4 dotted literals such as `127.0.0.1` (digits are
/// valid label characters).
fn is_valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_patterns() {
        for p in [
            "example.com",
            "*.example.com",
            "localhost",
            "127.0.0.1",
            "a-b.example.com",
            "xn--pple-43d.com",
        ] {
            assert!(is_valid_pattern(p), "{p} should be valid");
        }
    }

    #[test]
    fn invalid_patterns() {
        let sixty_four_char_label = "a".repeat(64);
        let cases: Vec<String> = vec![
            "".to_string(),
            "*".to_string(),
            "*.".to_string(),
            "**.example.com".to_string(),
            "foo.*.com".to_string(),
            "Example.com".to_string(),
            "https://example.com".to_string(),
            "example.com/path".to_string(),
            "example.com:8443".to_string(),
            "user@example.com".to_string(),
            ".example.com".to_string(),
            "example.com.".to_string(),
            "example..com".to_string(),
            "-foo.example.com".to_string(),
            "foo-.example.com".to_string(),
            sixty_four_char_label,
            "b\u{fc}cher.de".to_string(),
        ];
        for p in cases {
            assert!(!is_valid_pattern(&p), "{p} should be invalid");
        }
    }
}
