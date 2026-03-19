//! Integration tests for struct serialization and deserialization.

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
struct SimpleConfig {
    title: String,
    count: i32,
    enabled: bool,
    ratio: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct AppConfig {
    name: String,
    server: Server,
}

// ════════════════════════════════════════════════════════════════════════
// Basic struct tests
// ════════════════════════════════════════════════════════════════════════

deser_ok!(
    deserialize_simple_struct,
    SimpleConfig,
    indoc! {r#"
        title "My App"
        count 42
        enabled #true
        ratio 3.125
    "#},
    SimpleConfig {
        title: "My App".into(),
        count: 42,
        enabled: true,
        ratio: 3.125
    }
);

roundtrip!(
    serialize_simple_struct,
    SimpleConfig,
    SimpleConfig {
        title: "My App".into(),
        count: 42,
        enabled: true,
        ratio: 3.125,
    }
);

deser_ok!(
    deserialize_nested_struct,
    AppConfig,
    indoc! {r#"
        name "webapp"
        server {
            host "localhost"
            port 8080
        }
    "#},
    AppConfig {
        name: "webapp".into(),
        server: Server {
            host: "localhost".into(),
            port: 8080,
        },
    }
);

roundtrip!(
    serialize_nested_struct,
    AppConfig,
    AppConfig {
        name: "webapp".into(),
        server: Server {
            host: "localhost".into(),
            port: 8080,
        },
    }
);

// ── Deeply nested ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Level3 {
    value: String,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Level2 {
    inner: Level3,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Level1 {
    middle: Level2,
}

#[test]
fn deeply_nested() {
    let input = indoc! {r#"
        middle {
            inner {
                value "deep"
            }
        }
    "#};
    let val: Level1 = serde_kdl2::from_str(input).unwrap();
    assert_eq!(val.middle.inner.value, "deep");
    let output = serde_kdl2::to_string(&val).unwrap();
    let rt: Level1 = serde_kdl2::from_str(&output).unwrap();
    assert_eq!(val, rt);
}