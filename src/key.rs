use std::fmt;
use std::str::FromStr;

use serde::de;
use wayrs_utils::keyboard::xkb;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub keysym: xkb::Keysym,
    pub repr: String,
    pub mod_ctrl: bool,
    pub mod_alt: bool,
}

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut components = s.split('+');
        let key = components.next_back().unwrap_or(s);
        let keysym = to_keysym(key).ok_or_else(|| format!("invalid key '{key}'"))?;

        let mut mod_ctrl = false;
        let mut mod_alt = false;
        for modifier in components {
            if modifier.eq_ignore_ascii_case("ctrl") {
                mod_ctrl = true;
            } else if modifier.eq_ignore_ascii_case("alt") {
                mod_alt = true;
            } else {
                return Err(format!("unknown modifier '{modifier}"));
            }
        }

        Ok(Self {
            keysym,
            repr: s.to_owned(),
            mod_ctrl,
            mod_alt,
        })
    }
}

fn to_keysym(s: &str) -> Option<xkb::Keysym> {
    let mut chars = s.chars();
    let first_char = chars.next()?;

    let keysym = if chars.next().is_none() {
        xkb::utf32_to_keysym(first_char as u32)
    } else {
        xkb::keysym_from_name(s, xkb::KEYSYM_NO_FLAGS)
    };

    if keysym.raw() == xkb::keysyms::KEY_NoSymbol {
        None
    } else {
        Some(keysym)
    }
}

impl<'de> de::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct KeyVisitor;

        impl<'de> de::Visitor<'de> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("key")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}
