//! # serde-kdl2
//!
//! [Serde](https://serde.rs) integration for [KDL](https://kdl.dev), the
//! KDL Document Language.
//!
//! This crate provides `serialize` and `deserialize` support for KDL documents
//! using the [`kdl`](https://crates.io/crates/kdl) crate (v6, KDL v2 spec).
//!
//! ## Quick Start
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Config {
//!     title: String,
//!     count: i32,
//!     enabled: bool,
//! }
//!
//! let kdl_input = r#"
//! title "My App"
//! count 42
//! enabled #true
//! "#;
//!
//! // Deserialize
//! let config: Config = serde_kdl2::from_str(kdl_input).unwrap();
//! assert_eq!(config.title, "My App");
//! assert_eq!(config.count, 42);
//! assert_eq!(config.enabled, true);
//!
//! // Serialize
//! let output = serde_kdl2::to_string(&config).unwrap();
//! let roundtrip: Config = serde_kdl2::from_str(&output).unwrap();
//! assert_eq!(config, roundtrip);
//! ```
//!
//! ## Mapping Rules
//!
//! ### Structs and Maps
//!
//! Struct fields map to node names. Each field becomes a node whose name is
//! the field name and whose first argument is the value.
//!
//! ```kdl
//! title "My App"
//! count 42
//! enabled #true
//! ```
//!
//! ### Boolean Shorthand
//!
//! Boolean fields can use custom defaults for bare node names (without arguments)
//! using the `deserialize_with` attribute:
//!
//! ```rust
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     #[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_true")]
//!     enabled: bool,
//!     
//!     #[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_false")]
//!     disabled: bool,
//! }
//! ```
//!
//! ```kdl
//! enabled          // → true (due to bare_true)
//! disabled         // → false (due to bare_false)
//! enabled #false   // → false (explicit value overrides default)
//! disabled #true   // → true (explicit value overrides default)
//! ```
//!
//! ### Nested Structs
//!
//! Nested structs use children blocks:
//!
//! ```kdl
//! server {
//!     host "localhost"
//!     port 8080
//! }
//! ```
//!
//! ### Sequences
//!
//! Sequences of primitives use multiple arguments on a single node:
//!
//! ```kdl
//! tags "web" "rust" "config"
//! ```
//!
//! Sequences of structs use repeated nodes with the same name:
//!
//! ```kdl
//! server {
//!     host "localhost"
//!     port 8080
//! }
//! server {
//!     host "example.com"
//!     port 443
//! }
//! ```
//!
//! The `-` (dash) children convention is also supported for deserialization:
//!
//! ```kdl
//! items {
//!     - 1
//!     - 2
//!     - 3
//! }
//! ```
//!
//! ### Option
//!
//! `None` is represented by the absence of a node. `Some(value)` serializes
//! the inner value normally. `#null` arguments also deserialize as `None`.
//!
//! ### Enums
//!
//! Unit variants serialize as strings:
//!
//! ```kdl
//! color "Red"
//! ```
//!
//! Newtype, tuple, and struct variants use the variant name as a child node:
//!
//! ```kdl
//! shape {
//!     Circle {
//!         radius 5.0
//!     }
//! }
//! ```

pub mod de;
pub mod error;
pub mod ser;

pub use de::{from_doc, from_str};
pub use error::Error;
pub use ser::{to_doc, to_string, to_string_pretty};

/// Serde helper functions for custom boolean defaults with bare node names.
pub mod bool_defaults {
    use serde::{de, Deserializer};

    /// Deserializes a boolean field where bare node names default to `true`.
    /// 
    /// Use with `#[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_true")]`.
    /// 
    /// # Examples
    /// 
    /// ```kdl
    /// enabled        // → true
    /// enabled #true  // → true
    /// enabled #false // → false
    /// ```
    pub fn bare_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        bare_default::<D, true>(deserializer)
    }

    /// Deserializes a boolean field where bare node names default to `false`.
    /// 
    /// Use with `#[serde(deserialize_with = "serde_kdl2::bool_defaults::bare_false")]`.
    /// 
    /// # Examples
    /// 
    /// ```kdl
    /// disabled        // → false
    /// disabled #true  // → true
    /// disabled #false // → false
    /// ```
    pub fn bare_false<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        bare_default::<D, false>(deserializer)
    }

    fn bare_default<'de, D, const DEFAULT: bool>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BareDefaultVisitor<const DEFAULT: bool>;

        impl<'de, const DEFAULT: bool> de::Visitor<'de> for BareDefaultVisitor<DEFAULT> {
            type Value = bool;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a boolean value or bare node name")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DEFAULT)
            }
        }

        deserializer.deserialize_any(BareDefaultVisitor::<DEFAULT>)
    }
}
