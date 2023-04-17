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
        pangocairo::update_layout(context, &self.layout);

        context.save()?;

        context.translate(options.x, options.y);

        options.fg_color.apply(context);
        context.translate(0.0, (options.height - self.height) * 0.5);
        pangocairo::show_layout(context, &self.layout);

        context.restore()?;

        if std::env::var("WLR_WHICH_KEY_LAYOUT_DEBUG").as_deref() == Ok("1") {
            Color::from_rgba(255, 0, 0, 255).apply(context);
            context.rectangle(options.x, options.y, self.width, options.height);
            context.set_line_width(1.0);
            context.stroke().unwrap();
        }

        Ok(())
    }
}
