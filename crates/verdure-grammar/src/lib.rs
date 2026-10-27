//! JSON Schema → GBNF, and a validator over the same schema subset.
//!
//! This is the pure half of `extract()` (knowledge/26): a vendor hands Verdure
//! *their* schema (note sections, FactsR-style facts, an XForm's fields, a
//! lesson-observation record) and gets back JSON that matches it. Two tools,
//! one subset:
//!
//! * [`to_gbnf`] renders the schema as a llama.cpp grammar so a constrained
//!   sampler can only emit matching JSON. The grammar is **not** injected into
//!   the prompt by llama.cpp (its README says so), which is why
//!   `verdure-core::extract` also writes the schema into the prompt.
//! * [`validate`] checks a JSON value against the schema and names every
//!   violation by path, which is what the repair retry feeds back to the model.
//!
//! Subset (anything else is reported in [`Grammar::warnings`], never silently
//! accepted): `object` with `properties` / `required` / `additionalProperties:
//! false`, `string` (+ `enum`, `const`, `minLength`, `maxLength`), `number`,
//! `integer` (+ `minimum`, `maximum`), `boolean`, `null`, `array` (+ `items`,
//! `minItems`, `maxItems`), `anyOf` / `oneOf`, `$ref` to `#/$defs/…` or
//! `#/definitions/…`, and nullable types written as `["string", "null"]`.
//!
//! One deliberate deviation from llama.cpp's own converter: in the grammar,
//! **every declared property is emitted, in schema order**, and an optional one
//! is allowed to be `null`. A fixed layout is easier for a 1–4B model than
//! "any subset, any order", and "not mentioned" has one spelling. Null-valued
//! optional properties are stripped by [`strip_optional_nulls`] before
//! validation, so the vendor still receives standards-conforming JSON.

mod gbnf;
mod shape;
mod validate;

pub use gbnf::{to_gbnf, Grammar, GrammarError};
pub use shape::shape;
pub use validate::{strip_optional_nulls, validate, Violation};
