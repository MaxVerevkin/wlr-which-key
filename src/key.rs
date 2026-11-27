use std::fmt;
use std::str::FromStr;

use serde::de;
use wayrs_utils::keyboard::xkb;

#[derive(Clone)]
pub struct Key {
    any_of: Vec<SingleKey>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModifierState {
    pub mod_ctrl: bool,
    pub mod_alt: bool,
    pub mod_mod4: bool,
}

impl ModifierState {
    pub fn from_xkb_state(xkb: &xkb::State) -> Self {
        Self {
            mod_ctrl: xkb.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE),
            mod_alt: xkb.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE),
            mod_mod4: xkb.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE),
        }
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SingleKey {
    pub keysym: xkb::Keysym,
    pub repr: String,
    pub modifiers: ModifierState,
}

impl Key {
    pub fn matches(&self, sym: xkb::Keysym, modifiers: ModifierState) -> bool {
        self.any_of
            .iter()
            .any(|key| key.modifiers == modifiers && key.keysym == sym)
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
                modifiers: Default::default(),
            });
        }

        let mut components = s.split('+');
        let key = components.next_back().unwrap_or(s);
        let keysym = to_keysym(key).ok_or_else(|| format!("invalid key '{key}'"))?;

        let mut modifiers = ModifierState::default();
        for modifier in components {
            if modifier.eq_ignore_ascii_case("ctrl") {
                modifiers.mod_ctrl = true;
            } else if modifier.eq_ignore_ascii_case("alt") {
                modifiers.mod_alt = true;
            } else if modifier.eq_ignore_ascii_case("mod4") || modifier.eq_ignore_ascii_case("logo")
            {
                modifiers.mod_mod4 = true;
            } else {
                return Err(format!("unknown modifier '{modifier}"));
            }
        }

        Ok(Self {
            keysym,
            repr: s.to_owned(),
            modifiers,
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

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;
    use wayrs_utils::keyboard::xkb;

    #[rstest]
    #[case::cap_a("A", SingleKey { keysym: xkb::Keysym::new(65), repr: "A".to_string(), modifiers: ModifierState::default() })]
    #[case::low_a("a", SingleKey { keysym: xkb::Keysym::new(97), repr: "a".to_string(), modifiers: ModifierState::default() })]
    #[case::plus("+", SingleKey { keysym: xkb::Keysym::plus, repr: "+".to_string(), modifiers: ModifierState::default() })]
    #[case::ctrl_b("ctrl+b", SingleKey { keysym: xkb::Keysym::new(98), repr: "ctrl+b".to_string(), modifiers: ModifierState { mod_ctrl: true, ..Default::default() } })]
    #[case::ctrl_alt_c("ctrl+alt+c", SingleKey { keysym: xkb::Keysym::new(99), repr: "ctrl+alt+c".to_string(), modifiers: ModifierState { mod_ctrl: true, mod_alt: true, ..Default::default() } })]
    #[case::ctrl_alt_mod4_d("ctrl+alt+mod4+d", SingleKey { keysym: xkb::Keysym::new(100), repr: "ctrl+alt+mod4+d".to_string(), modifiers: ModifierState { mod_ctrl: true, mod_alt: true, mod_mod4: true } })]
    #[case::logo_e("logo+e", SingleKey { keysym: xkb::Keysym::new(101), repr: "logo+e".to_string(), modifiers: ModifierState { mod_mod4: true, ..Default::default() }})]
    fn it_parses_strings_to_single_key(#[case] input: &str, #[case] expected: SingleKey) {
        let actual: SingleKey = input.parse().unwrap();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::aa("aa")]
    #[case::fff("fff")]
    #[case::clrt_g("clrt+g")]
    #[case::h_ctrl("h+ctrl")]
    //  TODO: Currently this would deserialize, but it should probably return an error
    // #[case::ctrl_ctrl_i("ctrl+ctrl+i")]
    fn it_rejects_invalid_single_key_strings(#[case] input: &str) {
        let result = input.parse::<SingleKey>();
        assert!(result.is_err(), "Expected {result:?} to be an error");
    }
}
