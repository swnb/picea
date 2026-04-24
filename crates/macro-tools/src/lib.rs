//! Standalone derive helpers for small Rust data types.
//!
//! This crate intentionally stays independent from the `picea` runtime crate.
//! It only exposes three proc macros:
//!
//! - [`Accessors`]: generate explicit getters, mutable references, and setters.
//! - [`Builder`]: generate a small `TypeBuilder` for named-field structs.
//! - [`Deref`]: forward `Deref` and, when requested, `DerefMut` to one field.
//!
//! The macros prefer a narrow, explicit attribute surface over backwards
//! compatibility. Historical `Fields`, `Shape`, `wasm_config`, `r`, `w`, and
//! `shared` behavior is intentionally not implemented here.
//!
//! # Validation
//!
//! Run the crate-local test suite with:
//!
//! ```text
//! rtk proxy cargo test -p picea-macro-tools
//! ```
//!
//! The tests use `trybuild` fixtures to lock both accepted forms and
//! compile-time diagnostics for unsupported attributes.

mod accessors;
mod builder;
mod deref;

use accessors::macro_accessors;
use builder::macro_builder;
use deref::macro_deref;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derives `core::ops::Deref` for exactly one named field.
///
/// Mark the target field with `#[deref]` to generate `Deref`, or with
/// `#[deref(mut)]` to also generate `DerefMut`.
///
/// ```ignore
/// use picea_macro_tools::Deref;
///
/// #[derive(Deref)]
/// struct Wrapper {
///     #[deref(mut)]
///     value: Vec<i32>,
/// }
/// ```
///
/// Only named-field structs are supported. Tuple structs, unit structs, enums,
/// missing target fields, and multiple target fields all fail at compile time.
#[proc_macro_derive(Deref, attributes(deref))]
pub fn deref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_deref(input)
}

/// Derives a `TypeBuilder` companion for a named-field struct.
///
/// Generated builders expose `new()`, chainable setters, and
/// `build() -> Result<Type, &'static str>`.
///
/// Supported field attributes:
///
/// - `#[builder(default)]` uses `Default::default()` when the field is unset.
/// - `#[builder(default = expr)]` evaluates `expr` when the field is unset.
/// - `#[builder(skip, default)]` or `#[builder(skip, default = expr)]` omits the
///   setter and fills the field from its default expression.
///
/// The derive does not implement `Default` for the original struct.
#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_builder(input)
}

/// Derives field access methods for named-field structs.
///
/// Struct-level `#[accessor(...)]` options define defaults; field-level options
/// override or extend them. Supported options are `get`, `get(copy)`,
/// `get(clone)`, `mut`, `set`, `set(into)`, `skip`, and `vis(...)`.
///
/// `get` returns `&T`, `get(copy)` returns `T` by copy, `get(clone)` returns a
/// cloned `T`, `mut` generates `field_mut()`, and `set` generates
/// `set_field(value)`. `skip` is field-only and cannot be combined with other
/// accessor options.
#[proc_macro_derive(Accessors, attributes(accessor))]
pub fn accessors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_accessors(input)
}
