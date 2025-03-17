use std::fmt;
use std::str::FromStr;

use serde::de;
use wayrs_utils::keyboard::xkb;

#[derive(Clone)]
pub struct Key {
    any_of: Vec<SingleKey>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SingleKey {
    keysym: xkb::Keysym,
    repr: String,
    mod_ctrl: bool,
    mod_alt: bool,
}

impl Key {
    pub fn matches(&self, sym: xkb::Keysym, ctrl: bool, alt: bool) -> bool {
        self.any_of
            .iter()
            .any(|key| key.mod_ctrl == ctrl && key.mod_alt == alt && key.keysym == sym)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, key) in self.any_of.iter().enumerate() {
            f.write_str(&key.repr)?;
            if i + 1 != self.any_of.len() {
                f.write_str(" | ")?
            }
        }
        Ok(())
    }
}

impl From<SingleKey> for Key {
    fn from(value: SingleKey) -> Self {
        Self {
            any_of: vec![value],
        }
    }
}

impl FromStr for SingleKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "+" {
            return Ok(Self {
                keysym: xkb::Keysym::plus,
                repr: String::from("+"),
                mod_ctrl: false,
                mod_alt: false,
            });
        }

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
                formatter.write_str("a key or a list of keys")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Key::from(s.parse::<SingleKey>().map_err(E::custom)?))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut any_of = Vec::new();
                while let Some(next) = seq.next_element()? {
                    any_of.push(next);
                }
                Ok(Key { any_of })
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
}

impl<'de> de::Deserialize<'de> for SingleKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct KeyVisitor;

        impl de::Visitor<'_> for KeyVisitor {
            type Value = SingleKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a key")
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
