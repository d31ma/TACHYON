use super::{ApplicationConfig, Failure, invalid};
use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

struct BoundedValue {
    depth: usize,
}

impl BoundedValue {
    const MAX_DEPTH: usize = 16;
    const MAX_KEY_BYTES: usize = 128;

    const fn child(&self) -> Self {
        Self {
            depth: self.depth + 1,
        }
    }

    fn is_rejected_key(values: &Map<String, Value>, key: &str) -> bool {
        values.len() >= ApplicationConfig::MAX_KEYS
            || key.len() > Self::MAX_KEY_BYTES
            || matches!(key, "__proto__" | "prototype" | "constructor")
            || values.contains_key(key)
    }
}

impl<'de> DeserializeSeed<'de> for BoundedValue {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > Self::MAX_DEPTH {
            return Err(de::Error::custom("configuration depth limit"));
        }
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for BoundedValue {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON with unique keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(self.child())? {
            if values.len() >= ApplicationConfig::MAX_KEYS {
                return Err(de::Error::custom("array item limit"));
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if Self::is_rejected_key(&values, &key) {
                return Err(de::Error::custom("invalid or duplicate configuration key"));
            }
            let value = object.next_value_seed(self.child())?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value, Failure> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = BoundedValue { depth: 0 }
        .deserialize(&mut deserializer)
        .map_err(|_| {
            invalid("Application configuration is not valid bounded JSON with unique, safe keys.")
        })?;
    deserializer
        .end()
        .map_err(|_| invalid("Application configuration contains trailing content."))?;
    validate_bounds(&value)?;
    Ok(value)
}

pub(super) fn validate_bounds(value: &Value) -> Result<(), Failure> {
    let mut keys = 0;
    count_keys(value, &mut keys)
}

fn count_keys(value: &Value, keys: &mut usize) -> Result<(), Failure> {
    match value {
        Value::Object(object) => {
            *keys += object.len();
            if *keys > ApplicationConfig::MAX_KEYS {
                return Err(invalid(
                    "Application configuration exceeds 128 total object keys.",
                ));
            }
            for value in object.values() {
                count_keys(value, keys)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                count_keys(value, keys)?;
            }
        }
        _ => {}
    }
    Ok(())
}
