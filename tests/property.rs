use hegel::TestCase;
use hegel::generators::{
    Generator, booleans, floats, from_regex, integers, optional, sampled_from, text, vecs,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Generator for f64 values that can roundtrip through KDL.
///
/// NaN is excluded because `NaN != NaN`, so roundtrip equality assertions
/// always fail. Infinity is excluded because KDL has no infinity literal —
/// the serializer would need to encode it as a string, which changes the type.
fn finite_f64() -> impl Generator<f64> {
    floats::<f64>().allow_nan(false).allow_infinity(false)
}

/// Generator for valid KDL node-name identifiers (non-empty, starts with a letter).
fn kdl_identifier() -> impl Generator<String> {
    from_regex("[a-zA-Z][a-zA-Z0-9_-]{0,15}").fullmatch(true)
}

// ── Flat struct roundtrip ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FlatStruct {
    name: String,
    count: i32,
    enabled: bool,
    ratio: f64,
}

#[hegel::test]
fn flat_struct_roundtrip(tc: TestCase) {
    let val = FlatStruct {
        name: tc.draw(text()),
        count: tc.draw(integers()),
        enabled: tc.draw(booleans()),
        ratio: tc.draw(finite_f64()),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: FlatStruct = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Nested struct roundtrip ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Inner {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Outer {
    label: String,
    inner: Inner,
}

#[hegel::test]
fn nested_struct_roundtrip(tc: TestCase) {
    let val = Outer {
        label: tc.draw(text()),
        inner: Inner {
            host: tc.draw(text()),
            port: tc.draw(integers()),
        },
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: Outer = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Vec of primitives roundtrip ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithVecStrings {
    label: String,
    tags: Vec<String>,
}

#[hegel::test]
fn vec_strings_roundtrip(tc: TestCase) {
    let val = WithVecStrings {
        label: tc.draw(text()),
        tags: tc.draw(vecs(text())),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithVecStrings = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithVecInts {
    label: String,
    numbers: Vec<i64>,
}

#[hegel::test]
fn vec_ints_roundtrip(tc: TestCase) {
    let val = WithVecInts {
        label: tc.draw(text()),
        numbers: tc.draw(vecs(integers::<i64>())),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithVecInts = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Vec of structs roundtrip ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    name: String,
    value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithItems {
    title: String,
    item: Vec<Item>,
}

#[hegel::test]
fn vec_structs_roundtrip(tc: TestCase) {
    let count = tc.draw(integers::<usize>().max_value(10));
    let mut items = Vec::new();
    for _ in 0..count {
        items.push(Item {
            name: tc.draw(text()),
            value: tc.draw(integers()),
        });
    }
    let val = WithItems {
        title: tc.draw(text()),
        item: items,
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithItems = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Option fields roundtrip ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
struct WithOptions {
    required: String,
    #[serde(default)]
    maybe_str: Option<String>,
    #[serde(default)]
    maybe_num: Option<i64>,
    #[serde(default)]
    maybe_bool: Option<bool>,
}

#[hegel::test]
fn option_fields_roundtrip(tc: TestCase) {
    let val = WithOptions {
        required: tc.draw(text()),
        maybe_str: tc.draw(optional(text())),
        maybe_num: tc.draw(optional(integers::<i64>())),
        maybe_bool: tc.draw(optional(booleans())),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithOptions = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Enum roundtrip ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithEnum {
    label: String,
    color: Color,
}

#[hegel::test]
fn unit_enum_roundtrip(tc: TestCase) {
    let val = WithEnum {
        label: tc.draw(text()),
        color: tc.draw(sampled_from(vec![Color::Red, Color::Green, Color::Blue])),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithEnum = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Complex enum roundtrip ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Shape {
    Circle { radius: i32 },
    Rectangle { width: i32, height: i32 },
    Point,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithShape {
    name: String,
    shape: Shape,
}

#[hegel::test]
fn complex_enum_roundtrip(tc: TestCase) {
    let variant = tc.draw(integers::<u8>().min_value(0).max_value(2));
    let shape = match variant {
        0 => Shape::Circle {
            radius: tc.draw(integers()),
        },
        1 => Shape::Rectangle {
            width: tc.draw(integers()),
            height: tc.draw(integers()),
        },
        _ => Shape::Point,
    };
    let val = WithShape {
        name: tc.draw(text()),
        shape,
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithShape = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── BTreeMap roundtrip ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithMap {
    title: String,
    metadata: BTreeMap<String, String>,
}

#[hegel::test]
fn btreemap_roundtrip(tc: TestCase) {
    let keys = tc.draw(vecs(kdl_identifier()).unique(true));
    let mut metadata = BTreeMap::new();
    for key in keys {
        metadata.insert(key, tc.draw(text()));
    }
    let val = WithMap {
        title: tc.draw(text()),
        metadata,
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithMap = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Integer types roundtrip ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct IntegerTypes {
    a: i8,
    b: i16,
    c: i32,
    d: i64,
    e: u8,
    f: u16,
    g: u32,
    h: u64,
}

#[hegel::test]
fn integer_types_roundtrip(tc: TestCase) {
    let val = IntegerTypes {
        a: tc.draw(integers()),
        b: tc.draw(integers()),
        c: tc.draw(integers()),
        d: tc.draw(integers()),
        e: tc.draw(integers()),
        f: tc.draw(integers()),
        g: tc.draw(integers()),
        h: tc.draw(integers()),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: IntegerTypes = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Bool roundtrip ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Flags {
    a: bool,
    b: bool,
    c: bool,
}

#[hegel::test]
fn bool_roundtrip(tc: TestCase) {
    let val = Flags {
        a: tc.draw(booleans()),
        b: tc.draw(booleans()),
        c: tc.draw(booleans()),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: Flags = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Tuple roundtrip ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithTuple {
    label: String,
    pair: (i32, i32),
}

#[hegel::test]
fn tuple_roundtrip(tc: TestCase) {
    let val = WithTuple {
        label: tc.draw(text()),
        pair: (tc.draw(integers()), tc.draw(integers())),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithTuple = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Deeply nested roundtrip ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Level3 {
    value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Level2 {
    tag: String,
    level3: Level3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Level1 {
    name: String,
    level2: Level2,
}

#[hegel::test]
fn deeply_nested_roundtrip(tc: TestCase) {
    let val = Level1 {
        name: tc.draw(text()),
        level2: Level2 {
            tag: tc.draw(text()),
            level3: Level3 {
                value: tc.draw(integers()),
            },
        },
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: Level1 = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Serialize then deserialize never panics ──────────────────────────────

/// Ensure serialization of arbitrary valid structs never panics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct KitchenSink {
    s: String,
    i: i64,
    b: bool,
    f: f64,
    tags: Vec<String>,
    #[serde(default)]
    opt: Option<i32>,
}

#[hegel::test]
fn kitchen_sink_roundtrip(tc: TestCase) {
    let val = KitchenSink {
        s: tc.draw(text()),
        i: tc.draw(integers()),
        b: tc.draw(booleans()),
        f: tc.draw(finite_f64()),
        tags: tc.draw(vecs(text())),
        opt: tc.draw(optional(integers::<i32>())),
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: KitchenSink = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Pretty printing roundtrip ────────────────────────────────────────────

#[hegel::test]
fn pretty_print_roundtrip(tc: TestCase) {
    let val = FlatStruct {
        name: tc.draw(text()),
        count: tc.draw(integers()),
        enabled: tc.draw(booleans()),
        ratio: tc.draw(finite_f64()),
    };
    let serialized = serde_kdl2::to_string_pretty(&val).unwrap();
    let deserialized: FlatStruct = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}

// ── Newtype enum variant roundtrip ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Wrapper {
    Text(String),
    Number(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithNewtype {
    label: String,
    wrapped: Wrapper,
}

#[hegel::test]
fn newtype_enum_roundtrip(tc: TestCase) {
    let variant = tc.draw(booleans());
    let wrapped = if variant {
        Wrapper::Text(tc.draw(text()))
    } else {
        Wrapper::Number(tc.draw(integers()))
    };
    let val = WithNewtype {
        label: tc.draw(text()),
        wrapped,
    };
    let serialized = serde_kdl2::to_string(&val).unwrap();
    let deserialized: WithNewtype = serde_kdl2::from_str(&serialized).unwrap();
    assert_eq!(val, deserialized);
}
