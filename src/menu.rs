use crate::config::{self, Config};
use crate::text::{self, ComputedText};
use anyhow::Result;
use pangocairo::{cairo, pango};

pub struct Menu {
    key_col_width: f64,
    val_col_width: f64,
    item_height: f64,
    items: Vec<MenuItem>,
    separator: ComputedText,
}

struct MenuItem {
    action: Action,
    key_comp: ComputedText,
    val_comp: ComputedText,
    key_str: String,
}

pub enum Action {
    Exec(String),
    Submenu(Menu),
}

impl Menu {
    pub fn new(config: &Config) -> Self {
        let context = pango::Context::new();
        let fontmap = pangocairo::FontMap::new();
        context.set_font_map(Some(&fontmap));
        Self::new_with_centext(&config.font, &context, &config.menu, config)
    }

    pub fn new_with_centext(
        font: &pango::FontDescription,
        context: &pango::Context,
        entries: &config::Entries,
        config: &Config,
    ) -> Self {
        let separator = ComputedText::new(&config.separator, context, font);

        let mut items = Vec::new();
        for (key, entry) in &entries.0 {
            match entry {
                config::Entry::Cmd { cmd, desc } => {
                    items.push(MenuItem {
                        action: Action::Exec(cmd.into()),
                        key_comp: ComputedText::new(key, context, font),
                        val_comp: ComputedText::new(desc, context, font),
                        key_str: key.into(),
                    });
                }
                config::Entry::Recursive {
                    submenu: entries,
                    desc,
                } => {
                    items.push(MenuItem {
                        action: Action::Submenu(Self::new_with_centext(
                            font, context, entries, config,
                        )),
                        key_comp: ComputedText::new(key, context, font),
                        val_comp: ComputedText::new(&format!("+{desc}"), context, font),
                        key_str: key.into(),
                    });
                }
            }
        }

        let mut item_height = separator.height;
        for item in &items {
            if item.key_comp.height > item_height {
                item_height = item.key_comp.height;
            }
            if item.val_comp.height > item_height {
                item_height = item.val_comp.height;
            }
        }

        let key_col_width = items
            .iter()
            .map(|i| i.key_comp.width)
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();
        let val_col_width = items
            .iter()
            .map(|i| i.val_comp.width)
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();

        Self {
            key_col_width,
            val_col_width,
            item_height,
            separator,
            items,
        }
    }

    pub fn width(&self) -> f64 {
        self.key_col_width + self.val_col_width + self.separator.width
    }

    pub fn height(&self) -> f64 {
        self.item_height * self.items.len() as f64
    }

    pub fn render(
        &self,
        config: &config::Config,
        cairo_ctx: &cairo::Context,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let fg_color = config.color;

        for (i, comp) in self.items.iter().enumerate() {
            comp.key_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + self.key_col_width - comp.key_comp.width,
                    y: dy + self.item_height * (i as f64),
                    fg_color,
                    height: self.item_height,
                },
            )?;
            self.separator.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + self.key_col_width,
                    y: dy + self.item_height * (i as f64),
                    fg_color,
                    height: self.item_height,
                },
            )?;
            comp.val_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + self.key_col_width + self.separator.width,
                    y: dy + self.item_height * (i as f64),
                    fg_color,
                    height: self.item_height,
                },
            )?;
        }

        Ok(())
    }

    pub fn get_action(&mut self, key: &str) -> Option<Action> {
        let i = self.items.iter().position(|i| i.key_str == key)?;
        Some(self.items.remove(i).action)
    }
}
