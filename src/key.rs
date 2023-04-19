use std::fmt;
use std::str::FromStr;

use serde::de;
use wayrs_utils::keyboard::xkb;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Char(String),
    Keysym { keysym: xkb::Keysym, repr: String },
}

impl Key {
    pub fn repr(&self) -> &str {
        match self {
            Self::Char(c) => c,
            Self::Keysym { repr, .. } => repr,
        }
    }
}

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let Some(_first_char) = chars.next() else { return Err(()) };

        if chars.next().is_none() {
            return Ok(Self::Char(s.to_owned()));
        }

        let keysym = xkb::keysym_from_name(s, xkb::KEYSYM_NO_FLAGS);

        if keysym == xkb::KEY_NoSymbol {
            Err(())
        } else {
            Ok(Self::Keysym {
                keysym,
                repr: s.to_owned(),
            })
        }
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
                s.parse()
                    .map_err(|_| E::custom(format!("unknown key '{s}'")))
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}
