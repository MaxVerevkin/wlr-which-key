mod anchor;
mod compat;
mod entry;
mod font;

use std::env;
use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use smart_default::SmartDefault;

pub use self::anchor::ConfigAnchor;
pub use self::entry::Entry;
pub use self::font::Font;
use crate::color::Color;

#[derive(Deserialize, SmartDefault)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    #[default(Color::from_rgba_hex(0x282828ff))]
    pub background: Color,
    #[default(Color::from_rgba_hex(0xfbf1c7ff))]
    pub color: Color,
    #[default(Color::from_rgba_hex(0x8ec07cff))]
    pub border: Color,

    pub anchor: ConfigAnchor,
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,

    #[default(Font::new("monospace 10"))]
    pub font: Font,
    #[default(" ➜ ".into())]
    pub separator: String,
    #[default(4.0)]
    pub border_width: f64,
    #[default(20.0)]
    pub corner_r: f64,
    pub padding: Option<f64>,
    pub rows_per_column: Option<usize>,
    pub column_padding: Option<f64>,

    pub inhibit_compositor_keyboard_shortcuts: bool,
    pub auto_kbd_layout: bool,

    pub menu: Vec<Entry>,
}

impl Config {
    pub fn new(name: &str) -> Result<Self> {
        let mut config_path = config_dir().context("Cound not find config directory")?;
        config_path.push("wlr-which-key");
        config_path.push(name);
        config_path.set_extension("yaml");

        if !config_path.exists() {
            bail!("config file not found: {}", config_path.display());
        }

        let config_str = read_to_string(config_path).context("Failed to read configuration")?;

        match serde_yaml::from_str::<Self>(&config_str)
            .context("Failed to deserialize configuration")
        {
            Ok(config) => Ok(config),
            Err(err) => match serde_yaml::from_str::<compat::Config>(&config_str) {
                Ok(compat) => {
                    eprintln!(
                        "Warning: using the old config format, which will be removed in a future version."
                    );
                    Ok(compat.into())
                }
                Err(_compat_err) => Err(err),
            },
        }
    }

    pub fn padding(&self) -> f64 {
        self.padding.unwrap_or(self.corner_r)
    }

    pub fn column_padding(&self) -> f64 {
        self.column_padding.unwrap_or_else(|| self.padding())
    }
}

fn config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(env::var_os("HOME")?).join(".config")))
}
