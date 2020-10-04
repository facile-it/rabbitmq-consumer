use std::str::FromStr;

use serde::{de, Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(untagged)]
enum BoolOrString {
    Bool(bool),
    Str(String),
}
pub fn bool_or_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match BoolOrString::deserialize(deserializer)? {
        BoolOrString::Bool(v) => Ok(v),
        BoolOrString::Str(v) => bool::from_str(&v).map_err(de::Error::custom),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum I32OrString {
    Integer(i32),
    Str(String),
}

pub fn i32_or_string<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    match I32OrString::deserialize(deserializer)? {
        I32OrString::Integer(v) => Ok(v),
        I32OrString::Str(v) => i32::from_str(&v).map_err(de::Error::custom),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum U16OrString {
    Integer(u16),
    Str(String),
}

pub fn u16_or_string<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    match U16OrString::deserialize(deserializer)? {
        U16OrString::Integer(v) => Ok(v),
        U16OrString::Str(v) => u16::from_str(&v).map_err(de::Error::custom),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Unsigned64OrString {
    Integer(u64),
    Str(String),
}

pub fn u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    match Unsigned64OrString::deserialize(deserializer)? {
        Unsigned64OrString::Integer(v) => Ok(v),
        Unsigned64OrString::Str(v) => u64::from_str(&v).map_err(de::Error::custom),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OptionIntegerOrString {
    Integer(i32),
    Str(String),
}

pub fn option_i32_or_string<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    match i32_or_string(deserializer) {
        Ok(value) => Ok(Some(value)),
        _ => Ok(None),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OptionUnsigned64OrString {
    Integer(u64),
    Str(String),
}

pub fn option_u64_or_string<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    match u64_or_string(deserializer) {
        Ok(value) => Ok(Some(value)),
        _ => Ok(None),
    }
}
