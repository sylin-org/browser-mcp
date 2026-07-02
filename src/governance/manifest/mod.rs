//! Policy manifest -- the org policy / grants document (ADR-0020). Domain-agnostic core:
//! generic over any policy doc, names no browser type.
//!
//! Today this module holds only [`identity`] (ADR-0020 commitment 5: name, version, and a
//! computed content hash, so every logged decision is attributable to the exact policy
//! version that made it). The manifest engine (task G12: parsing, grants, source selection)
//! lands here too, alongside identity, once it ships.

pub mod identity;
