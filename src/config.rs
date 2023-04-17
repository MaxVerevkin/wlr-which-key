use crate::color::Color;
use anyhow::{Context, Result};
use dirs_next::config_dir;
use indexmap::IndexMap;
use pangocairo::pango::FontDescription;
use serde::{de, Deserialize};
use std::fmt;
use std::fs::read_to_string;
use std::ops::Deref;
use wayrs_protocols::wlr_layer_shell_unstable_v1::zwlr_layer_surface_v1::Anchor;

#[derive(Deserialize, Default)]
#[serde(transparent)]
pub struct Entries(pub IndexMap<String, Entry>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    pub background: Color,
    pub color: Color,
    pub border: Color,
    pub anchor: ConfigAnchor,

    pub font: Font,
    pub border_width: f64,
    pub corner_r: f64,

    pub menu: Entries,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Entry {
    Cmd { cmd: String, desc: String },
    Recursive { submenu: Box<Entries>, desc: String },
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // A kind of gruvbox theme
            background: Color::from_rgba_hex(0x282828ff),
            color: Color::from_rgba_hex(0xfbf1c7ff),
            border: Color::from_rgba_hex(0x8ec07cff),
            border_width: 4.0,
            corner_r: 20.0,
            font: Font::new("monospace 10"),
            menu: Default::default(),
            anchor: ConfigAnchor::default(),
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        Ok(match config_dir() {
            Some(mut config_path) => {
                config_path.push("wlr-which-key");
                config_path.push("config.yaml");
                if config_path.exists() {
                    let config =
                        read_to_string(config_path).context("Failed to read configuration")?;
                    serde_yaml::from_str(&config).context("Failed to deserialize configuration")?
                } else {
                    info!("Using default configuration");
                    Self::default()
                }
            }
            None => {
                warn!("Could not find the configuration path");
                info!("Using default configuration");
                Self::default()
            }
        })
    }
}

pub struct Font(pub FontDescription);

impl Font {
    pub fn new(desc: &str) -> Self {
        Self(FontDescription::from_string(desc))
    }
}

impl Deref for Font {
    type Target = FontDescription;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> de::Deserialize<'de> for Font {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct FontVisitor;

        impl<'de> de::Visitor<'de> for FontVisitor {
            type Value = Font;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("font description")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Font::new(s))
            }
        }

        deserializer.deserialize_str(FontVisitor)
    }
}

/// Light wrapper around `Anchor` which also supports the "no anchor" value.
///
/// This type is also requires to derive `Deserialize` for the foreign type.
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum ConfigAnchor {
    #[default]
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Convert this anchor into the type expected by `wayrs`.
impl From<ConfigAnchor> for Anchor {
    fn from(value: ConfigAnchor) -> Self {
        match value {
            ConfigAnchor::Center => Anchor::empty(),
            ConfigAnchor::Top => Anchor::Top,
            ConfigAnchor::Bottom => Anchor::Bottom,
            ConfigAnchor::Left => Anchor::Left,
            ConfigAnchor::Right => Anchor::Right,
            ConfigAnchor::TopLeft => Anchor::Top | Anchor::Left,
            ConfigAnchor::TopRight => Anchor::Top | Anchor::Right,
            ConfigAnchor::BottomLeft => Anchor::Bottom | Anchor::Left,
            ConfigAnchor::BottomRight => Anchor::Bottom | Anchor::Right,
        }
    }
}
