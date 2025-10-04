use std::{
    ffi::{CString, NulError},
    str::FromStr,
};

use serde::de;

#[derive(Default)]
pub struct Namespace(pub CString);

impl Namespace {
    pub fn new(namespace: CString) -> Self {
        Self(namespace)
    }
}

impl FromStr for Namespace {
    type Err = NulError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(CString::new(s)?))
    }
}

impl<'de> de::Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct NamespaceVisitor;

        impl de::Visitor<'_> for NamespaceVisitor {
            type Value = Namespace;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("namespace name")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(|_| {
                    E::custom(format!("'{}' contains a null character", s.escape_debug()))
                })
            }
        }

        deserializer.deserialize_str(NamespaceVisitor)
    }
}
