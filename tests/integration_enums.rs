//! Integration tests for enum serialization and deserialization.

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

macro_rules! roundtrip {
    ($name:ident, $ty:ty, $val:expr) => {
        #[test]
        fn $name() {
            let val: $ty = $val;
            let output = serde_kdl2::to_string(&val).unwrap();
            let rt: $ty = serde_kdl2::from_str(&output).unwrap();
            assert_eq!(val, rt);
        }
    };
}

// Shared types
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Colored {
    name: String,
    color: Color,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

// ── Enum variants ──────────────────────────────────────────────────────

deser_ok!(
    deserialize_unit_variant,
    Colored,
    indoc! {r#"
        name "widget"
        color "Red"
    "#},
    Colored {
        name: "widget".into(),
        color: Color::Red
    }
);

roundtrip!(
    serialize_unit_variant,
    Colored,
    Colored {
        name: "widget".into(),
        color: Color::Green
    }
);

// ── Struct variant enum ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Drawing {
    name: String,
    shape: Shape,
}

deser_ok!(
    deserialize_struct_variant,
    Drawing,
    indoc! {r#"
        name "my drawing"
        shape {
            Circle {
                radius 5.0
            }
        }
    "#},
    Drawing {
        name: "my drawing".into(),
        shape: Shape::Circle { radius: 5.0 }
    }
);

roundtrip!(
    serialize_struct_variant,
    Drawing,
    Drawing {
        name: "my drawing".into(),
        shape: Shape::Rectangle {
            width: 10.0,
            height: 20.0
        },
    }
);

// ── Newtype variant enum ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Wrapper {
    Text(String),
    Number(i64),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Wrapped {
    value: Wrapper,
}

deser_ok!(
    deserialize_newtype_variant,
    Wrapped,
    indoc! {r#"
        value {
            Text "hello"
        }
    "#},
    Wrapped {
        value: Wrapper::Text(String::from("hello"))
    }
);

roundtrip!(
    serialize_newtype_variant,
    Wrapped,
    Wrapped {
        value: Wrapper::Text(String::from("hello"))
    }
);
