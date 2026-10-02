mod anchor;
mod entry;
mod font;
mod namespace;

use std::env;
use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use smart_default::SmartDefault;

pub use self::anchor::ConfigAnchor;
pub use self::entry::Entry;
pub use self::font::Font;
pub use self::namespace::Namespace;
use crate::color::Color;

#[derive(Deserialize, SmartDefault)]
#[serde(deny_unknown_fields, default)]
pub struct ConfigBuilder {
    #[serde(flatten)]
    theme: ThemeBuilder,

    inhibit_compositor_keyboard_shortcuts: bool,
    auto_kbd_layout: bool,

    menu: Vec<Entry>,

    #[default(Namespace::new(c"wlr_which_key".to_owned()))]
    namespace: Namespace,
}

impl ConfigBuilder {
    pub fn new(name: &str) -> Result<Self> {
        let mut config_path = config_dir().context("Cound not find config directory")?;
        config_path.push("wlr-which-key");
        config_path.push(name);
        config_path.set_extension("yaml");

        if !config_path.exists() {
            bail!("config file not found: {}", config_path.display());
        }

        let config_str = read_to_string(config_path).context("Failed to read configuration")?;

        serde_yaml::from_str::<Self>(&config_str).context("Failed to deserialize configuration")
    }

    pub fn with_theme(&mut self, theme: ThemeBuilder) {
        self.theme.merge(theme);
    }

    pub fn build(self) -> Config {
        let Self {
            theme,
            inhibit_compositor_keyboard_shortcuts,
            auto_kbd_layout,
            menu,
            namespace,
        } = self;

        Config {
            theme: theme.build(),
            inhibit_compositor_keyboard_shortcuts,
            auto_kbd_layout,
            menu,
            namespace,
        }
    }
}

pub struct Theme {
    pub background: Color,
    pub color: Color,
    pub border: Color,

    pub anchor: ConfigAnchor,
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,

    pub font: Font,
    pub separator: String,
    pub border_width: f64,
    pub corner_r: f64,
    pub padding: f64,
    pub rows_per_column: Option<usize>,
    pub column_padding: f64,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ThemeBuilder {
    pub background: Option<Color>,
    pub color: Option<Color>,
    pub border: Option<Color>,

    pub anchor: Option<ConfigAnchor>,
    pub margin_top: Option<i32>,
    pub margin_right: Option<i32>,
    pub margin_bottom: Option<i32>,
    pub margin_left: Option<i32>,

    pub font: Option<Font>,
    pub separator: Option<String>,
    pub border_width: Option<f64>,
    pub corner_r: Option<f64>,
    pub padding: Option<f64>,
    pub rows_per_column: Option<usize>,
    pub column_padding: Option<f64>,
}

impl ThemeBuilder {
    pub fn new(name: &str) -> Result<Self> {
        let mut theme_path = config_dir().context("Cound not find config directory")?;
        theme_path.push("wlr-which-key");
        theme_path.push(name);
        theme_path.set_extension("yaml");

        if !theme_path.exists() {
            bail!("theme file not found: {}", theme_path.display());
        }

        let theme_str = read_to_string(theme_path).context("Failed to read theme")?;

        serde_yaml::from_str::<Self>(&theme_str).context("Failed to deserialize theme")
    }

    pub fn merge(&mut self, other: Self) {
        macro_rules! merge {
            ($($field_name:ident),*) => {
                $( self.$field_name = self.$field_name.take().or_else(|| other.$field_name); )*
            };
        }

        merge!(
            background,
            color,
            border,
            anchor,
            margin_top,
            margin_right,
            margin_bottom,
            margin_left,
            font,
            separator,
            border_width,
            corner_r,
            padding,
            rows_per_column,
            column_padding
        );
    }

    pub fn build(self) -> Theme {
        macro_rules! build_theme {
            (normal_init {$($normal_field:ident: $val:expr),*},$(($theme_field:ident, $default:expr)),*) => {
                Theme {
                    $($normal_field: $val),*,
                    $($theme_field: self.$theme_field.unwrap_or($default)),*
                }
            }
        }

        let corner_r = self.corner_r.unwrap_or(20.0);

        let padding = self.padding.unwrap_or(corner_r);
        build_theme!(
            normal_init {
                rows_per_column: self.rows_per_column
            },
            (background, Color::from_rgba_hex(0x282828ff)),
            (color, Color::from_rgba_hex(0xfbf1c7ff)),
            (border, Color::from_rgba_hex(0x8ec07cff)),
            (anchor, ConfigAnchor::default()),
            (margin_top, 0),
            (margin_right, 0),
            (margin_bottom, 0),
            (margin_left, 0),
            (font, Font::new("monospace 10")),
            (separator, " ➜ ".into()),
            (border_width, 4.0),
            (corner_r, corner_r),
            (padding, padding),
            (column_padding, padding)
        )
    }
}

pub struct Config {
    pub theme: Theme,

    pub inhibit_compositor_keyboard_shortcuts: bool,
    pub auto_kbd_layout: bool,

    pub menu: Vec<Entry>,

    pub namespace: Namespace,
}

fn config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(env::var_os("HOME")?).join(".config")))
}
