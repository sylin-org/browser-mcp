// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Generic ASCII-armored block: a base64 payload wrapped in `-----BEGIN <label>-----` /
//! `-----END <label>-----` markers at 64 columns.
//!
//! Two signed-envelope kinds share it: the license (ADR-0028 Decision 11, label
//! `GHOSTLIGHT LICENSE`) and the managed policy bundle (ADR-0055, label `GHOSTLIGHT POLICY`). The
//! armored payload decodes to the EXACT envelope bytes, so the armored and raw-JSON forms verify
//! identically. Reuses [`crate::b64`] (lean-internals posture: no base64 crate).

/// Wrap `payload` bytes as an ASCII-armored block for `label` (for example `"GHOSTLIGHT POLICY"`),
/// base64 wrapped at 64 columns.
pub fn wrap(label: &str, payload: &[u8]) -> String {
    let b64 = crate::b64::encode(payload);
    let mut out = String::with_capacity(b64.len() + 2 * label.len() + 32);
    out.push_str("-----BEGIN ");
    out.push_str(label);
    out.push_str("-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
        out.push('\n');
    }
    out.push_str("-----END ");
    out.push_str(label);
    out.push_str("-----\n");
    out
}

/// Extract the payload bytes from an ASCII-armored block for `label`, or `None` if the markers are
/// absent or the body is not valid base64. Whitespace between the markers is ignored.
pub fn unwrap(label: &str, block: &str) -> Option<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = block.find(&begin)? + begin.len();
    let stop = block[start..].find(&end)? + start;
    let body: String = block[start..stop].split_whitespace().collect();
    crate::b64::decode(&body)
}

/// True when `block` contains the begin marker for `label` (vs. a raw JSON envelope).
pub fn is_armored(label: &str, block: &str) -> bool {
    block.contains(&format!("-----BEGIN {label}-----"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_round_trip() {
        let payload = b"the exact envelope bytes \x00\x01\x02\xff";
        let block = wrap("GHOSTLIGHT POLICY", payload);
        assert!(is_armored("GHOSTLIGHT POLICY", &block));
        assert!(block.contains("-----BEGIN GHOSTLIGHT POLICY-----"));
        assert!(block.contains("-----END GHOSTLIGHT POLICY-----"));
        assert_eq!(unwrap("GHOSTLIGHT POLICY", &block).unwrap(), payload);
    }

    #[test]
    fn unwrap_ignores_surrounding_and_internal_whitespace() {
        let block = wrap("GHOSTLIGHT LICENSE", b"hello world payload bytes");
        let messy = format!("preamble line\n\n{block}\ntrailing line\n");
        assert_eq!(
            unwrap("GHOSTLIGHT LICENSE", &messy).unwrap(),
            b"hello world payload bytes"
        );
    }

    #[test]
    fn wrong_label_does_not_match() {
        let block = wrap("GHOSTLIGHT LICENSE", b"x");
        assert!(!is_armored("GHOSTLIGHT POLICY", &block));
        assert!(unwrap("GHOSTLIGHT POLICY", &block).is_none());
    }

    #[test]
    fn missing_markers_return_none() {
        assert!(unwrap("GHOSTLIGHT POLICY", "no markers here").is_none());
    }
}
