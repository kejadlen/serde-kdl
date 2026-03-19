//! Integration tests for Option field handling.

use indoc::indoc;
use serde::{Deserialize, Serialize};

// Test macros
macro_rules! deser_ok {
    ($name:ident, $ty:ty, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let val: $ty = serde_kdl2::from_str($input).unwrap();
            assert_eq!(val, $expected);
        }
    };
}

// ── Option fields ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct OptionalFields {
    required: String,
    optional: Option<String>,
}

deser_ok!(
    deserialize_option_present,
    OptionalFields,
    indoc! {r#"
        required "hello"
        optional "world"
    "#},
    OptionalFields {
        required: String::from("hello"),
        optional: Some(String::from("world"))
    }
);

deser_ok!(
    deserialize_option_absent,
    OptionalFields,
    r#"required "hello""#,
    OptionalFields {
        required: String::from("hello"),
        optional: None
    }
);

deser_ok!(
    deserialize_option_null,
    OptionalFields,
    indoc! {r#"
        required "hello"
        optional #null
    "#},
    OptionalFields {
        required: String::from("hello"),
        optional: None
    }
);

#[test]
fn serialize_option() {
    let with = OptionalFields {
        required: String::from("hello"),
        optional: Some(String::from("world")),
    };
    let output = serde_kdl2::to_string(&with).unwrap();
    assert!(output.contains("optional"));
    let rt: OptionalFields = serde_kdl2::from_str(&output).unwrap();
    assert_eq!(with, rt);

    let without = OptionalFields {
        required: String::from("hello"),
        optional: None,
    };
    let output = serde_kdl2::to_string(&without).unwrap();
    assert!(!output.contains("optional"), "None options should not serialize");
    let rt: OptionalFields = serde_kdl2::from_str(&output).unwrap();
    assert_eq!(without, rt);
}