use anyhow::{bail, Result};
use pangocairo::{cairo, pango};
use wayrs_utils::keyboard::xkb;

use crate::config::{self, Config};
use crate::key::Key;
use crate::text::{self, ComputedText};

pub struct Menu {
    pages: Vec<MenuPage>,
    cur_page: usize,
    separator: ComputedText,
}

struct MenuPage {
    key_col_width: f64,
    val_col_width: f64,
    item_height: f64,
    items: Vec<MenuItem>,
    parent: Option<usize>,
}

struct MenuItem {
    action: Action,
    key_comp: ComputedText,
    val_comp: ComputedText,
    key: Key,
}

#[derive(Clone)]
pub enum Action {
    Quit,
    Exec(String),
    Submenu(usize),
}

impl Menu {
    pub fn new(config: &Config) -> Result<Self> {
        let context = pango::Context::new();
        let fontmap = pangocairo::FontMap::new();
        context.set_font_map(Some(&fontmap));

        let mut this = Self {
            pages: Vec::new(),
            cur_page: 0,
            separator: ComputedText::new(&config.separator, &context, &config.font),
        };

        this.push_page(&context, &config.menu, config, None)?;

        Ok(this)
    }

    fn push_page(
        &mut self,
        context: &pango::Context,
        entries: &config::Entries,
        config: &Config,
        parent: Option<usize>,
    ) -> Result<usize> {
        if entries.0.is_empty() {
            bail!("Empty menu pages are not allowed");
        }

        let cur_page = self.pages.len();

        self.pages.push(MenuPage {
            key_col_width: 0.0,
            val_col_width: 0.0,
            item_height: self.separator.height,
            items: Vec::new(),
            parent,
        });

        for (key, entry) in &entries.0 {
            let item = match entry {
                config::Entry::Cmd { cmd, desc } => MenuItem {
                    action: Action::Exec(cmd.into()),
                    key_comp: ComputedText::new(&key.repr, context, &config.font),
                    val_comp: ComputedText::new(desc, context, &config.font),
                    key: key.clone(),
                },
                config::Entry::Recursive {
                    submenu: entries,
                    desc,
                } => {
                    let new_page = self.push_page(context, entries, config, Some(cur_page))?;
                    MenuItem {
                        action: Action::Submenu(new_page),
                        key_comp: ComputedText::new(&key.repr, context, &config.font),
                        val_comp: ComputedText::new(&format!("+{desc}"), context, &config.font),
                        key: key.clone(),
                    }
                }
            };

            if item.key_comp.height > self.pages[cur_page].item_height {
                self.pages[cur_page].item_height = item.key_comp.height;
            }
            if item.key_comp.width > self.pages[cur_page].key_col_width {
                self.pages[cur_page].key_col_width = item.key_comp.width;
            }

            if item.val_comp.height > self.pages[cur_page].item_height {
                self.pages[cur_page].item_height = item.val_comp.height;
            }
            if item.val_comp.width > self.pages[cur_page].val_col_width {
                self.pages[cur_page].val_col_width = item.val_comp.width;
            }

            self.pages[cur_page].items.push(item);
        }

        Ok(cur_page)
    }

    pub fn width(&self) -> f64 {
        let page = &self.pages[self.cur_page];
        page.key_col_width + page.val_col_width + self.separator.width
    }

    pub fn height(&self) -> f64 {
        let page = &self.pages[self.cur_page];
        page.item_height * page.items.len() as f64
    }

    pub fn render(
        &self,
        config: &config::Config,
        cairo_ctx: &cairo::Context,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let page = &self.pages[self.cur_page];
        let fg_color = config.color;

        for (i, comp) in page.items.iter().enumerate() {
            comp.key_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + page.key_col_width - comp.key_comp.width,
                    y: dy + page.item_height * (i as f64),
                    fg_color,
                    height: page.item_height,
                },
            )?;
            self.separator.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + page.key_col_width,
                    y: dy + page.item_height * (i as f64),
                    fg_color,
                    height: page.item_height,
                },
            )?;
            comp.val_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + page.key_col_width + self.separator.width,
                    y: dy + page.item_height * (i as f64),
                    fg_color,
                    height: page.item_height,
                },
            )?;
        }

        Ok(())
    }

    pub fn get_action(&self, xkb: &xkb::State, keycode: xkb::Keycode) -> Option<Action> {
        let page = &self.pages[self.cur_page];
        let sym = xkb.key_get_one_sym(keycode);

        let mod_alt = xkb.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE);
        let mod_ctrl = xkb.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE);

        let item_i = page.items.iter().position(|i| {
            i.key.mod_ctrl == mod_ctrl && i.key.mod_alt == mod_alt && i.key.keysym == sym
        });

        if let Some(item_i) = item_i {
            return Some(page.items[item_i].action.clone());
        }

        match sym {
            xkb::Keysym::Escape => {
                return Some(Action::Quit);
            }
            xkb::Keysym::bracketleft | xkb::Keysym::g if mod_ctrl => {
                return Some(Action::Quit);
            }
            xkb::Keysym::BackSpace => {
                if let Some(parent) = page.parent {
                    return Some(Action::Submenu(parent));
                }
            }
            _ => (),
        }

        None
    }

    pub fn set_page(&mut self, page: usize) {
        self.cur_page = page;
    }
}
