use indexmap::IndexMap;
use serde::Deserialize;
use smart_default::SmartDefault;

use crate::color::Color;
use crate::key::SingleKey;

use super::{ConfigAnchor, Font};

#[derive(Deserialize, Default)]
#[serde(transparent)]
pub struct Entries(pub IndexMap<SingleKey, Entry>);

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
    // defaults to `corner_r`
    pub padding: Option<f64>,

    pub menu: Entries,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Entry {
    Cmd {
        cmd: String,
        desc: String,
        #[serde(default)]
        keep_open: bool,
    },
    Recursive {
        submenu: Entries,
        desc: String,
    },
}

impl From<Config> for super::Config {
    fn from(value: Config) -> Self {
        fn map_entries(value: Entries) -> Vec<super::Entry> {
            value
                .0
                .into_iter()
                .map(|(key, entry)| match entry {
                    Entry::Cmd {
                        cmd,
                        desc,
                        keep_open,
                    } => super::Entry::Cmd {
                        key: key.into(),
                        cmd,
                        desc,
                        keep_open,
                    },
                    Entry::Recursive { submenu, desc } => super::Entry::Recursive {
                        key: key.into(),
                        submenu: map_entries(submenu),
                        desc,
                    },
                })
                .collect()
        }

        Self {
            background: value.background,
            color: value.color,
            border: value.border,
            anchor: value.anchor,
            margin_top: value.margin_top,
            margin_right: value.margin_right,
            margin_bottom: value.margin_bottom,
            margin_left: value.margin_left,
            font: value.font,
            separator: value.separator,
            border_width: value.border_width,
            corner_r: value.corner_r,
            padding: value.padding,
            rows_per_column: None,
            column_padding: None,
            menu: map_entries(value.menu),
            inhibit_compositor_keyboard_shortcuts: false,
        }
    }
}
