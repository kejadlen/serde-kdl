//! Serializer implementation for KDL.
//!
//! # Mapping Rules
//!
//! - **Struct/Map** → KDL document. Each field becomes a node.
//! - **Primitives** → Node argument: `name "value"`, `count 42`.
//! - **Nested struct** → Node with children block: `nested { field "val" }`.
//! - **Vec of primitives** → Single node, multiple arguments: `tags "a" "b" "c"`.
//! - **Vec of structs** → Multiple nodes with the same name.
//! - **Option::None** → Node omitted. **Option::Some** → Serialize inner value.
//! - **Enum (unit variant)** → String: `"VariantName"`.
//! - **Enum (newtype)** → Child node: `VariantName value`.
//! - **Enum (struct)** → Child node with children: `VariantName { field val }`.

use kdl::{KdlDocument, KdlEntry, KdlEntryFormat, KdlNode, KdlValue};
use serde::ser::{self, Serialize};

use crate::error::{Error, Result};

/// Returns true if `c` is a character that the `kdl` crate (v6.5) writes
/// verbatim but the KDL v2 spec forbids in unescaped form inside quoted
/// strings.
///
/// The kdl crate escapes: `\\`, `"`, `\n` (0x0A), `\r` (0x0D), `\t`
/// (0x09), `\b` (0x08), `\f` (0x0C). All other C0 controls, DEL, C1
/// controls, and Unicode newline characters are written raw.
fn is_unescaped_by_kdl(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        // C0 controls not handled by the kdl crate.
        0x00..=0x07 | 0x0B | 0x0E..=0x1F
        // DEL.
        | 0x7F
        // C1 controls (includes NEL at 0x85).
        | 0x80..=0x9F
        // Unicode line/paragraph separators treated as newlines by KDL.
        | 0x2028..=0x2029
    )
}

/// Build a `KdlEntry` for a string value, escaping control characters that
/// the `kdl` crate's `Display` doesn't handle.
///
/// The KDL v2 spec forbids unescaped control characters (U+0000–U+0008,
/// U+000E–U+001F, U+007F, and others) inside quoted strings. The `kdl`
/// crate (v6.5) only escapes `\n`, `\r`, `\t`, `\b`, `\f`, `\\`, and `"`,
/// writing all other characters verbatim. This produces invalid KDL for
/// strings containing other control characters.
///
/// We work around this by setting `value_repr` on the entry to a properly
/// escaped quoted string.
fn string_entry(s: &str) -> KdlEntry {
    let needs_escape = s.chars().any(is_unescaped_by_kdl);

    let mut entry = KdlEntry::new(KdlValue::String(s.to_string()));

    if needs_escape {
        let mut repr = String::with_capacity(s.len() + 2);
        repr.push('"');
        for c in s.chars() {
            match c {
                '\\' => repr.push_str("\\\\"),
                '"' => repr.push_str("\\\""),
                '\n' => repr.push_str("\\n"),
                '\r' => repr.push_str("\\r"),
                '\t' => repr.push_str("\\t"),
                '\u{08}' => repr.push_str("\\b"),
                '\u{0C}' => repr.push_str("\\f"),
                c if is_unescaped_by_kdl(c) => {
                    repr.push_str(&format!("\\u{{{:x}}}", c as u32));
                }
                c => repr.push(c),
            }
        }
        repr.push('"');
        entry.set_format(KdlEntryFormat {
            value_repr: repr,
            leading: " ".into(),
            // Prevent autoformat() from clearing the escaped repr.
            autoformat_keep: true,
            ..Default::default()
        });
    }

    entry
}

/// Serialize a value to a KDL string.
pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let doc = to_doc(value)?;
    Ok(doc.to_string())
}

/// Serialize a value to a KDL string with auto-formatting.
pub fn to_string_pretty<T: Serialize>(value: &T) -> Result<String> {
    let mut doc = to_doc(value)?;
    doc.autoformat();
    Ok(doc.to_string())
}

/// Serialize a value to a [`KdlDocument`].
pub fn to_doc<T: Serialize>(value: &T) -> Result<KdlDocument> {
    let v = value.serialize(ValueSerializer)?;
    value_to_doc(v)
}

// ---------------------------------------------------------------------------
// Intermediate value representation
// ---------------------------------------------------------------------------

/// An intermediate representation that bridges serde's data model and KDL's
/// tree structure. We serialize into this first, then convert to KDL.
#[derive(Debug, Clone)]
enum Value {
    /// Represents `Option::None` — the field should be omitted entirely.
    None,
    /// Represents `()` and unit structs — serializes as `#null`.
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),
    Seq(Vec<Value>),
    Map(Vec<(String, Value)>),
}

impl Value {
    fn is_primitive(&self) -> bool {
        matches!(
            self,
            Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Float(_) | Value::String(_)
        )
    }

    fn to_kdl_value(&self) -> Option<KdlValue> {
        match self {
            Value::Null => Some(KdlValue::Null),
            Value::Bool(b) => Some(KdlValue::Bool(*b)),
            Value::Integer(i) => Some(KdlValue::Integer(*i)),
            Value::Float(f) => Some(KdlValue::Float(*f)),
            Value::String(_) => unreachable!("strings use string_entry, not to_kdl_value"),
            _ => unreachable!("to_kdl_value is only called on primitive values"),
        }
    }
}

// ---------------------------------------------------------------------------
// Value → KDL conversion
// ---------------------------------------------------------------------------

fn value_to_doc(value: Value) -> Result<KdlDocument> {
    match value {
        Value::Map(entries) => {
            let mut doc = KdlDocument::new();
            for (key, val) in entries {
                let nodes = value_to_nodes(&key, val)?;
                for node in nodes {
                    doc.nodes_mut().push(node);
                }
            }
            Ok(doc)
        }
        _ => Err(Error::TopLevelNotStruct),
    }
}

/// Convert a key-value pair to one or more KDL nodes.
fn value_to_nodes(name: &str, value: Value) -> Result<Vec<KdlNode>> {
    match value {
        // None/Null reach value_to_nodes through the mixed-sequence `-`
        // children path (e.g., Vec<Option<Vec<T>>> with a None element).
        Value::None | Value::Null => {
            let mut node = KdlNode::new(name);
            node.push(KdlEntry::new(KdlValue::Null));
            Ok(vec![node])
        }
        Value::Bool(b) => {
            let mut node = KdlNode::new(name);
            node.push(KdlEntry::new(KdlValue::Bool(b)));
            Ok(vec![node])
        }
        Value::Integer(i) => {
            let mut node = KdlNode::new(name);
            node.push(KdlEntry::new(KdlValue::Integer(i)));
            Ok(vec![node])
        }
        Value::Float(f) => {
            let mut node = KdlNode::new(name);
            node.push(KdlEntry::new(KdlValue::Float(f)));
            Ok(vec![node])
        }
        Value::String(s) => {
            let mut node = KdlNode::new(name);
            node.push(string_entry(&s));
            Ok(vec![node])
        }

        // Seq: if all elements are primitive → one node with multiple args
        //      if elements are maps → multiple nodes with same name
        //      mixed → use `-` children
        Value::Seq(elements) => {
            if elements.is_empty() {
                // Empty sequence → node with children block but no children
                let mut node = KdlNode::new(name);
                node.set_children(KdlDocument::new());
                return Ok(vec![node]);
            }

            let all_primitive = elements.iter().all(|e| e.is_primitive());
            let all_maps = elements.iter().all(|e| matches!(e, Value::Map(_)));

            if all_primitive {
                // Single node, multiple arguments
                let mut node = KdlNode::new(name);
                for elem in &elements {
                    match elem {
                        Value::String(s) => node.push(string_entry(s)),
                        _ => {
                            if let Some(kv) = elem.to_kdl_value() {
                                node.push(KdlEntry::new(kv));
                            }
                        }
                    }
                }
                Ok(vec![node])
            } else if all_maps {
                // Multiple nodes with the same name
                let mut nodes = Vec::new();
                for elem in elements {
                    if let Value::Map(entries) = elem {
                        let mut node = KdlNode::new(name);
                        let children_doc = map_entries_to_doc(entries)?;
                        node.set_children(children_doc);
                        nodes.push(node);
                    } else {
                        unreachable!("all_maps was true but element is not Map");
                    }
                }
                Ok(nodes)
            } else {
                // Mixed or nested sequences → use `-` children
                let mut node = KdlNode::new(name);
                let mut children = KdlDocument::new();
                for elem in elements {
                    let child_nodes = value_to_nodes("-", elem)?;
                    for cn in child_nodes {
                        children.nodes_mut().push(cn);
                    }
                }
                node.set_children(children);
                Ok(vec![node])
            }
        }

        // Map → node with children
        Value::Map(entries) => {
            let mut node = KdlNode::new(name);
            let children_doc = map_entries_to_doc(entries)?;
            node.set_children(children_doc);
            Ok(vec![node])
        }
    }
}

fn map_entries_to_doc(entries: Vec<(String, Value)>) -> Result<KdlDocument> {
    let mut doc = KdlDocument::new();
    for (k, v) in entries {
        let nodes = value_to_nodes(&k, v)?;
        for node in nodes {
            doc.nodes_mut().push(node);
        }
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// ValueSerializer: serde Serializer that produces a Value
// ---------------------------------------------------------------------------

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_i128(self, v: i128) -> Result<Value> {
        Ok(Value::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::Integer(v as i128))
    }

    fn serialize_u128(self, v: u128) -> Result<Value> {
        let i: i128 = v
            .try_into()
            .map_err(|_| Error::IntegerOutOfRange(v as i128))?;
        Ok(Value::Integer(i))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::Float(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Seq(
            v.iter().map(|b| Value::Integer(*b as i128)).collect(),
        ))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::None)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value> {
        let inner = value.serialize(ValueSerializer)?;
        Ok(Value::Map(vec![(variant.to_string(), inner)]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SerializeSeq> {
        Ok(SerializeSeq {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SerializeSeq> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SerializeSeq> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            variant: variant.to_string(),
            elements: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<SerializeMap> {
        Ok(SerializeMap {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            current_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<SerializeMap> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SerializeStructVariant> {
        Ok(SerializeStructVariant {
            variant: variant.to_string(),
            entries: Vec::with_capacity(len),
        })
    }
}

// ---------------------------------------------------------------------------
// Compound serializer types
// ---------------------------------------------------------------------------

struct SerializeSeq {
    elements: Vec<Value>,
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Seq(self.elements))
    }
}

impl ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

struct SerializeTupleVariant {
    variant: String,
    elements: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(vec![(self.variant, Value::Seq(self.elements))]))
    }
}

struct SerializeMap {
    entries: Vec<(String, Value)>,
    current_key: Option<String>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        let key_val = key.serialize(ValueSerializer)?;
        let key_str = match key_val {
            Value::String(s) => s,
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => b.to_string(),
            // cov-excl-start — requires a map key that serializes to
            // Null, Seq, or Map. Standard serde derive types (HashMap,
            // BTreeMap) only produce String/Integer/Bool keys.
            other => {
                return Err(Error::Unsupported(format!(
                    "map key must be a string, got {other:?}"
                )));
            } // cov-excl-stop
        };
        self.current_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self
            .current_key
            .take()
            .ok_or_else(|| Error::Message("serialize_value called before serialize_key".into()))?;
        let val = value.serialize(ValueSerializer)?;
        // Skip None values (they represent absent optional fields)
        if !matches!(val, Value::None) {
            self.entries.push((key, val));
        }
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.entries))
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let val = value.serialize(ValueSerializer)?;
        // Skip None values for struct fields (Option::None)
        if !matches!(val, Value::None) {
            self.entries.push((key.to_string(), val));
        }
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.entries))
    }
}

struct SerializeStructVariant {
    variant: String,
    entries: Vec<(String, Value)>,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let val = value.serialize(ValueSerializer)?;
        if !matches!(val, Value::None) {
            self.entries.push((key.to_string(), val));
        }
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(vec![(self.variant, Value::Map(self.entries))]))
    }
}
