use anyhow::{Context, bail};
use serde::Deserialize;

use crate::key::Key;

#[derive(Deserialize)]
#[serde(try_from = "RawEntry")]
pub enum Entry {
    Cmd {
        key: Key,
        cmd: String,
        desc: String,
        keep_open: bool,
    },
    Recursive {
        key: Key,
        submenu: Vec<Self>,
        desc: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    key: Key,
    desc: String,
    cmd: Option<String>,
    keep_open: Option<bool>,
    submenu: Option<Vec<Entry>>,
}

impl TryFrom<RawEntry> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: RawEntry) -> Result<Self, Self::Error> {
        if let Some(submenu) = value.submenu {
            if value.cmd.is_some() {
                bail!("cannot have both 'submenu' and 'cmd'");
            }
            if value.keep_open.is_some() {
                bail!("cannot have both 'submenu' and 'keep_open'");
            }
            Ok(Self::Recursive {
                key: value.key,
                submenu,
                desc: value.desc,
            })
        } else {
            Ok(Self::Cmd {
                key: value.key,
                cmd: value
                    .cmd
                    .context("either or 'submenu' or 'cmd' is required")?,
                desc: value.desc,
                keep_open: value.keep_open.unwrap_or(false),
            })
        }
    }
}
