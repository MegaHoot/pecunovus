use serde::{Serializer, Deserializer};
use serde::de::Error as DeError;

/// Serialize bytes as hex string
pub fn as_hex<S>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&hex::encode(bytes))
}

/// Deserialize hex string into bytes
pub fn from_hex<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    hex::decode(&s).map_err(D::Error::custom)
}
