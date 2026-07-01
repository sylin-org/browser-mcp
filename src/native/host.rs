//! Chrome native-messaging host protocol.
//!
//! Framing: a **4-byte little-endian `u32` length prefix** followed by exactly that many bytes of
//! UTF-8 JSON, on the process's stdin/stdout (which Chrome connects to when it launches this
//! executable as a native-messaging host). Implemented in Phase 1; see
//! `reference/ANALYSIS.md` sec 3 for the reference behavior we mirror.
