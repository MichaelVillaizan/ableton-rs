//! Conversion of JSON values sent by Live into typed Rust values.
//!
//! Plain data goes through serde. Live objects (tracks, clips, …) need the
//! client to be usable, which serde can't provide, so everything a property
//! can return implements [`FromWire`] instead.
//!
//! Scalars are decoded leniently: the Python side sometimes reports a
//! number where a boolean is expected (or vice versa), and Live happily
//! accepts either, so we do too.

use serde_json::Value;

use crate::Ableton;
use crate::error::{Error, Result};

/// A type that can be built from a JSON value sent by the Remote Script.
pub trait FromWire: Sized {
    /// Converts `value`, using `client` for any Live objects it contains.
    fn from_wire(client: &Ableton, value: Value) -> Result<Self>;
}

impl FromWire for Value {
    fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
        Ok(value)
    }
}

impl FromWire for () {
    fn from_wire(_: &Ableton, _: Value) -> Result<Self> {
        Ok(())
    }
}

impl FromWire for bool {
    fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
        match &value {
            Value::Bool(b) => Ok(*b),
            Value::Number(n) => Ok(n.as_f64().is_some_and(|n| n != 0.0)),
            _ => Err(Error::decode("a boolean", &value)),
        }
    }
}

impl FromWire for f64 {
    fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
        match &value {
            Value::Number(n) => n.as_f64().ok_or_else(|| Error::decode("a number", &value)),
            Value::Bool(b) => Ok(f64::from(u8::from(*b))),
            _ => Err(Error::decode("a number", &value)),
        }
    }
}

impl FromWire for f32 {
    fn from_wire(client: &Ableton, value: Value) -> Result<Self> {
        f64::from_wire(client, value).map(|n| n as f32)
    }
}

macro_rules! integer_from_wire {
    ($($int:ty),*) => {$(
        impl FromWire for $int {
            fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
                let wide = match &value {
                    Value::Number(n) => n
                        .as_i64()
                        .or_else(|| n.as_f64().map(|f| f.round() as i64)),
                    Value::Bool(b) => Some(i64::from(*b)),
                    _ => None,
                };
                wide.and_then(|n| <$int>::try_from(n).ok())
                    .ok_or_else(|| Error::decode(stringify!($int), &value))
            }
        }
    )*};
}

integer_from_wire!(i8, i16, i32, i64, u8, u16, u32, u64, usize);

impl FromWire for String {
    fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
        match value {
            Value::String(s) => Ok(s),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            other => Err(Error::decode("a string", &other)),
        }
    }
}

impl<T: FromWire> FromWire for Option<T> {
    fn from_wire(client: &Ableton, value: Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            value => T::from_wire(client, value).map(Some),
        }
    }
}

impl<T: FromWire> FromWire for Vec<T> {
    fn from_wire(client: &Ableton, value: Value) -> Result<Self> {
        match value {
            Value::Array(items) => items
                .into_iter()
                .map(|item| T::from_wire(client, item))
                .collect(),
            other => Err(Error::decode("an array", &other)),
        }
    }
}

/// Decodes a value with serde. Used by [`wire_via_serde!`].
pub(crate) fn from_serde<T: serde::de::DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|error| Error::Decode(error.to_string()))
}

/// Implements [`FromWire`] for plain data types through serde.
macro_rules! wire_via_serde {
    ($($ty:ty),* $(,)?) => {$(
        impl $crate::wire::FromWire for $ty {
            fn from_wire(_: &$crate::Ableton, value: ::serde_json::Value) -> $crate::Result<Self> {
                $crate::wire::from_serde(value)
            }
        }
    )*};
}

pub(crate) use wire_via_serde;

/// Encodes a value for a command argument.
pub(crate) fn to_value<T: serde::Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|error| Error::Encode(error.to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn decodes_scalars_leniently() {
        let client = Ableton::default();
        assert!(bool::from_wire(&client, json!(1)).unwrap());
        assert!(!bool::from_wire(&client, json!(0.0)).unwrap());
        assert_eq!(i32::from_wire(&client, json!(true)).unwrap(), 1);
        assert_eq!(i32::from_wire(&client, json!(4.0)).unwrap(), 4);
        assert_eq!(f64::from_wire(&client, json!(120)).unwrap(), 120.0);
        assert!(u8::from_wire(&client, json!(300)).is_err());
        assert!(String::from_wire(&client, json!(null)).is_err());
    }

    #[test]
    fn decodes_containers() {
        let client = Ableton::default();
        assert_eq!(
            Vec::<Option<i32>>::from_wire(&client, json!([1, null, 3])).unwrap(),
            vec![Some(1), None, Some(3)]
        );
        assert!(Vec::<i32>::from_wire(&client, json!({})).is_err());
    }
}
