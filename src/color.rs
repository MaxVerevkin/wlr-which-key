use pangocairo::cairo::Context;
use serde::de;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    pub fn apply(self, cr: &Context) {
        if self.alpha.is_nan() {
            cr.set_source_rgb(self.red, self.green, self.blue);
        } else {
            cr.set_source_rgba(self.red, self.green, self.blue, self.alpha);
        }
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            red: r as f64 / 255.0,
            green: g as f64 / 255.0,
            blue: b as f64 / 255.0,
            alpha: if a == 255 { f64::NAN } else { a as f64 / 255.0 },
        }
    }

    pub fn from_rgba_hex(hex: u32) -> Self {
        let r = (hex >> 24 & 0xFF) as u8;
        let g = (hex >> 16 & 0xFF) as u8;
        let b = (hex >> 8 & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;
        Self::from_rgba(r, g, b, a)
    }
}

impl FromStr for Color {
    type Err = ();

    fn from_str(color: &str) -> Result<Self, Self::Err> {
        let rgb = color.get(1..7).ok_or(())?;
        let rgb = u32::from_str_radix(rgb, 16).map_err(|_| ())?;
        let r = (rgb >> 16 & 0xFF) as u8;
        let g = (rgb >> 8 & 0xFF) as u8;
        let b = (rgb & 0xFF) as u8;

        let a = match color.get(7..9) {
            Some(a) => u8::from_str_radix(a, 16).map_err(|_| ())?,
            None => 255,
        };

        Ok(Self::from_rgba(r, g, b, a))
    }
}

impl<'de> de::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> de::Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("RBG or RGBA color (in hex)")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse()
                    .map_err(|_| E::custom(format!("'{s}' is not a valid RGB/RGBA color")))
            }
        }

        deserializer.deserialize_str(ColorVisitor)
    }
}
