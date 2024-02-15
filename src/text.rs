use crate::color::Color;
use anyhow::Result;
use pango::FontDescription;
use pangocairo::{cairo, pango};

#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    pub x: f64,
    pub y: f64,
    pub fg_color: Color,
    pub height: f64,
}

#[derive(Clone, Debug)]
pub struct ComputedText {
    pub layout: pango::Layout,
    pub width: f64,
    pub height: f64,
}

impl ComputedText {
    pub fn new(text: &str, context: &pango::Context, font: &FontDescription) -> Self {
        let layout = pango::Layout::new(context);
        layout.set_font_description(Some(font));
        layout.set_markup(text);

        let (width, height) = layout.pixel_size();

        ComputedText {
            layout,
            width: width as f64,
            height: height as f64,
        }
    }

    pub fn render(&self, context: &cairo::Context, options: RenderOptions) -> Result<()> {
        pangocairo::functions::update_layout(context, &self.layout);

        context.save()?;
        context.translate(options.x, options.y + (options.height - self.height) * 0.5);

        options.fg_color.apply(context);
        pangocairo::functions::show_layout(context, &self.layout);

        if std::env::var("WLR_WHICH_KEY_LAYOUT_DEBUG").as_deref() == Ok("1") {
            Color::from_rgba(255, 0, 0, 255).apply(context);
            context.rectangle(0.0, 0.0, self.width, self.height);
            context.set_line_width(1.0);
            context.stroke().unwrap();
        }

        context.restore()?;

        Ok(())
    }
}
