// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Secret-value redaction for page observation output (`content.security.secrets.redact`).
//!
//! The engine is truthful: the extension emits a secret field's real value using the marker
//! attribute `secret_value="..."` -- a neutral fact ("the page marks this field secret"), not a
//! decision. This module is the governance overlay. It **always** rewrites the marker back to a
//! normal `value="..."` (so the model never sees the marker) and, when the key is enabled, replaces
//! the value with `[value redacted]`. Because the marker is stripped whether or not redaction is on,
//! disabling the key yields the raw truth and enabling it yields safe-by-default output -- with no
//! change required in the engine.
//!
//! The transform is a dependency-free scan over the marker token; it does not parse or interpret
//! page content beyond locating the marker the engine placed (consistent with SPEC sec 9.5: no
//! semantic content inspection).

use serde_json::Value;

/// The marker attribute the engine emits for a secret field's value.
const MARKER: &str = "secret_value=\"";
/// What a redacted value is replaced with (matches the official extension's wording).
const REDACTED: &str = "value=\"[value redacted]\"";

/// Apply secret redaction in place to an MCP tool result. Text markers are rewritten and actionable
/// structured values carrying the extension's neutral `secret:true` marker are redacted when the
/// policy key is enabled. The marker itself never leaves the binary.
pub fn apply_to_result(result: &mut Value, redact: bool) {
    if let Some(items) = result.get_mut("content").and_then(Value::as_array_mut) {
        for item in items {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                let rewritten = apply_to_tree(text, redact);
                item["text"] = Value::String(rewritten);
            }
        }
    }
    if let Some(structured) = result.get_mut("structuredContent") {
        redact_structured(structured, redact);
    }
}

fn redact_structured(value: &mut Value, redact: bool) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_structured(item, redact);
            }
        }
        Value::Object(object) => {
            let secret = object.get("secret").and_then(Value::as_bool) == Some(true);
            if secret && redact && object.get("value").is_some_and(Value::is_string) {
                object.insert(
                    "value".to_string(),
                    Value::String("[value redacted]".to_string()),
                );
            }
            object.remove("secret");
            for child in object.values_mut() {
                redact_structured(child, redact);
            }
        }
        _ => {}
    }
}

/// Rewrite every `secret_value="X"` marker in `tree`. When `redact`, the value becomes
/// `[value redacted]`; otherwise the raw value `X` is preserved. The marker is always removed, so
/// the string returned never contains `secret_value=`.
pub fn apply_to_tree(tree: &str, redact: bool) -> String {
    let mut out = String::with_capacity(tree.len());
    let mut rest = tree;
    while let Some(pos) = rest.find(MARKER) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + MARKER.len()..];
        // The value ends at the next quote; if the tree was truncated mid-marker, treat the
        // remainder as the value so the raw marker can never leak into the output.
        let (raw, tail) = match after.find('"') {
            Some(end) => (&after[..end], &after[end + 1..]),
            None => (after, ""),
        };
        if redact {
            out.push_str(REDACTED);
        } else {
            out.push_str("value=\"");
            out.push_str(raw);
            out.push('"');
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const LINE: &str =
        "textbox \"Password\" [ref_3] secret_value=\"hunter2\" type=\"password\"\ntextbox \"User\" [ref_2] value=\"alice\" type=\"text\"";

    #[test]
    fn redacts_marked_values_and_removes_the_marker() {
        let out = apply_to_tree(LINE, true);
        assert!(
            !out.contains("secret_value="),
            "marker must be stripped: {out}"
        );
        assert!(
            !out.contains("hunter2"),
            "secret must not survive redaction: {out}"
        );
        assert!(out.contains("value=\"[value redacted]\""), "{out}");
        // Non-sensitive values are untouched.
        assert!(out.contains("value=\"alice\""), "{out}");
    }

    #[test]
    fn preserves_raw_values_when_disabled_but_still_removes_the_marker() {
        let out = apply_to_tree(LINE, false);
        assert!(
            !out.contains("secret_value="),
            "marker must be stripped even raw: {out}"
        );
        assert!(
            out.contains("value=\"hunter2\""),
            "raw value expected when disabled: {out}"
        );
        assert!(out.contains("value=\"alice\""), "{out}");
    }

    #[test]
    fn handles_multiple_markers_and_a_truncated_tail() {
        let s = "a secret_value=\"one\" b secret_value=\"two\" c secret_value=\"trunc";
        let red = apply_to_tree(s, true);
        assert_eq!(red.matches("[value redacted]").count(), 3, "{red}");
        assert!(!red.contains("secret_value="), "{red}");
        assert!(
            !red.contains("one") && !red.contains("two") && !red.contains("trunc"),
            "{red}"
        );
    }

    #[test]
    fn no_marker_is_a_passthrough() {
        let s = "link [ref_1] href=\"https://example.com\"\nbutton \"Go\" [ref_2]";
        assert_eq!(apply_to_tree(s, true), s);
        assert_eq!(apply_to_tree(s, false), s);
    }

    #[test]
    fn apply_to_result_rewrites_only_text_items() {
        let mut result = json!({
            "content": [
                { "type": "text", "text": "in secret_value=\"pw\" out" },
                { "type": "image", "data": "secret_value=\"not-text\"" }
            ]
        });
        apply_to_result(&mut result, true);
        assert_eq!(
            result["content"][0]["text"],
            "in value=\"[value redacted]\" out"
        );
        // Non-text items are left untouched (the marker only ever appears in read_page text).
        assert_eq!(result["content"][1]["data"], "secret_value=\"not-text\"");
    }

    #[test]
    fn apply_to_result_redacts_structured_secret_values_and_removes_marker() {
        let mut result = json!({
            "structuredContent": {
                "results": [
                    { "ref": "ref_1", "value": "hunter2", "secret": true },
                    { "ref": "ref_2", "value": "alice" }
                ]
            }
        });
        apply_to_result(&mut result, true);
        assert_eq!(
            result["structuredContent"]["results"][0]["value"],
            "[value redacted]"
        );
        assert!(result["structuredContent"]["results"][0]
            .get("secret")
            .is_none());
        assert_eq!(result["structuredContent"]["results"][1]["value"], "alice");
    }

    #[test]
    fn disabled_structured_redaction_keeps_truth_but_removes_marker() {
        let mut result = json!({ "structuredContent": { "target": {
            "ref": "ref_1", "value": "hunter2", "secret": true
        }}});
        apply_to_result(&mut result, false);
        assert_eq!(result["structuredContent"]["target"]["value"], "hunter2");
        assert!(result["structuredContent"]["target"]
            .get("secret")
            .is_none());
    }
}
