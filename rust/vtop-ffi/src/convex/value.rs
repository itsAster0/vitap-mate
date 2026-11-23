//! Convex values as plain JSON. The app sends and reads float64, strings, arrays, objects, null
//! and bytes (`vtop:fetch` answers a packed page as an `ArrayBuffer`), which Convex's JSON
//! format writes as themselves and `{"$bytes": base64}`; its other tagged forms
//! (`{"$integer": ..}`) are refused rather than passed on as objects.

use std::collections::BTreeMap;

use anyhow::bail;
use serde_json::Value;
use vtop_shared::base64::{Engine as _, engine::general_purpose::STANDARD};

/// A Convex value in Convex's JSON format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvexValue(pub Value);

impl ConvexValue {
    pub fn array(items: Vec<ConvexValue>) -> Self {
        Self(Value::Array(items.into_iter().map(|item| item.0).collect()))
    }

    pub fn object(fields: BTreeMap<String, ConvexValue>) -> Self {
        Self(Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key, value.0))
                .collect(),
        ))
    }
}

impl TryFrom<Value> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(mut value: Value) -> anyhow::Result<Self> {
        check(&value)?;
        whole_numbers(&mut value);
        Ok(Self(value))
    }
}

/// Convex numbers are float64 and arrive as `1.0`, which serde won't read into an integer field:
/// a whole float that an f64 holds exactly becomes an integer.
fn whole_numbers(value: &mut Value) {
    const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53
    match value {
        Value::Array(items) => items.iter_mut().for_each(whole_numbers),
        Value::Object(fields) => fields.values_mut().for_each(whole_numbers),
        Value::Number(number) if !number.is_i64() && !number.is_u64() => {
            if let Some(float) = number.as_f64()
                && float.fract() == 0.0
                && float.abs() <= EXACT
            {
                #[expect(clippy::cast_possible_truncation, reason = "whole and within 2^53")]
                let whole = float as i64;
                *value = Value::from(whole);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

impl From<ConvexValue> for Value {
    fn from(value: ConvexValue) -> Self {
        value.0
    }
}

fn check(value: &Value) -> anyhow::Result<()> {
    match value {
        Value::Array(items) => items.iter().try_for_each(check),
        Value::Object(fields) => {
            if fields.len() == 1
                && let Some(key) = fields.keys().next()
                && key.starts_with('$')
                && key != "$bytes"
            {
                bail!("unsupported Convex value {key}");
            }
            fields.values().try_for_each(check)
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(()),
    }
}

/// The bytes of a `{"$bytes": base64}` value; None for any other value.
pub(crate) fn bytes(value: &Value) -> Option<Vec<u8>> {
    let Value::Object(fields) = value else {
        return None;
    };
    let text = fields
        .get("$bytes")
        .filter(|_| fields.len() == 1)?
        .as_str()?;
    STANDARD.decode(text).ok()
}

/// A function's `ConvexError`: a generic message and the thrown data.
#[derive(Clone, PartialEq, Eq)]
pub struct ConvexError {
    pub message: String,
    pub data: ConvexValue,
}

impl std::fmt::Debug for ConvexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:#?}", self.message)
    }
}
