//! Cairo drawing shared by the live overlay and the final image, so what is
//! on screen is exactly what gets saved.

use std::error::Error;
use std::f64::consts::{PI, TAU};

use gtk::{cairo, pango};

use crate::editor::{Shape, Tool};
use crate::geometry::Rect;
use crate::image::Image;
use crate::palette::{ACCENT, ANNOTATION, SURFACE, TEXT};
use crate::settings::AfterSelect;
use crate::toolbar::{Button, Hover, SettingsPanel, Toolbar, TOOLTIP_PAD_X};

const STROKE: f64 = 3.0;
const HIGHLIGHT_STROKE: f64 = 20.0;
const HIGHLIGHT_ALPHA: f64 = 0.35;
const ARROW_HEAD: f64 = 18.0;
const ARROW_SPREAD: f64 = 0.45;
const TEXT_SIZE: f64 = 24.0;
const TEXT_HALO: f64 = 4.0;
const ICON_STROKE: f64 = 1.8;
const LABEL_SIZE: f64 = 14.0;
const DISABLED_ALPHA: f64 = 0.3;
const HOVER_ALPHA: f64 = 0.14;

type Drawn = Result<(), cairo::Error>;

fn set_color(cr: &cairo::Context, hex: u32, alpha: f64) {
    let [_, r, g, b] = hex.to_be_bytes();
    cr.set_source_rgba(f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0, alpha);
}

fn trace_points(cr: &cairo::Context, points: &[(i32, i32)]) {
    for (i, &(x, y)) in points.iter().enumerate() {
        if i == 0 {
            cr.move_to(f64::from(x), f64::from(y));
        } else {
            cr.line_to(f64::from(x), f64::from(y));
        }
    }
}

fn draw_arrow(cr: &cairo::Context, from: (i32, i32), to: (i32, i32)) -> Drawn {
    let (fx, fy) = (f64::from(from.0), f64::from(from.1));
    let (tx, ty) = (f64::from(to.0), f64::from(to.1));
    let angle = (ty - fy).atan2(tx - fx);
    let length = (tx - fx).hypot(ty - fy);
    // On a short arrow the head would otherwise overshoot the tail.
    let head = ARROW_HEAD.min(length);
    let corner = |side: f64| {
        (
            tx - head * (angle + side * ARROW_SPREAD).cos(),
            ty - head * (angle + side * ARROW_SPREAD).sin(),
        )
    };
    cr.move_to(fx, fy);
    cr.line_to(tx - head * 0.8 * angle.cos(), ty - head * 0.8 * angle.sin());
    cr.stroke()?;
    let (ax, ay) = corner(1.0);
    let (bx, by) = corner(-1.0);
    cr.move_to(tx, ty);
    cr.line_to(ax, ay);
    cr.line_to(bx, by);
    cr.close_path();
    cr.fill()
}

fn font(size: f64, bold: bool) -> pango::FontDescription {
    let mut description = pango::FontDescription::from_string(if bold { "Sans Bold" } else { "Sans" });
    description.set_absolute_size(size * f64::from(pango::SCALE));
    description
}

fn draw_text(cr: &cairo::Context, at: (i32, i32), text: &str, caret: bool) -> Drawn {
    let layout = pangocairo::functions::create_layout(cr);
    layout.set_font_description(Some(&font(TEXT_SIZE, true)));
    layout.set_text(text);
    let (x, y) = (f64::from(at.0), f64::from(at.1));

    // A dark halo keeps the text readable on light backgrounds.
    cr.move_to(x, y);
    pangocairo::functions::layout_path(cr, &layout);
    set_color(cr, SURFACE, 1.0);
    cr.set_line_width(TEXT_HALO);
    cr.stroke()?;

    set_color(cr, ANNOTATION, 1.0);
    cr.move_to(x, y);
    pangocairo::functions::show_layout(cr, &layout);
    if caret {
        let (width, height) = layout.pixel_size();
        cr.rectangle(x + f64::from(width) + 2.0, y, 2.0, f64::from(height));
        cr.fill()?;
    }
    Ok(())
}

/// Draws one annotation in global logical coordinates.
pub fn draw_shape(cr: &cairo::Context, shape: &Shape, caret: bool) -> Drawn {
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    cr.set_line_width(STROKE);
    set_color(cr, ANNOTATION, 1.0);
    match shape {
        Shape::Arrow { from, to } => draw_arrow(cr, *from, *to),
        Shape::Rectangle { a, b } => {
            let r = Rect::from_points(*a, *b);
            cr.rectangle(f64::from(r.x), f64::from(r.y), f64::from(r.w), f64::from(r.h));
            cr.stroke()
        }
        Shape::Freehand(points) => {
            trace_points(cr, points);
            cr.stroke()
        }
        Shape::Highlighter(points) => {
            set_color(cr, ANNOTATION, HIGHLIGHT_ALPHA);
            cr.set_line_width(HIGHLIGHT_STROKE);
            trace_points(cr, points);
            cr.stroke()
        }
        Shape::Text { at, text } => draw_text(cr, *at, text, caret),
    }
}

/// Where the cropped image sits, for drawing annotations onto it.
pub struct Capture {
    /// The crop, in pixels of the full frame.
    pub region: Rect,
    pub shapes: Vec<Shape>,
    pub bounds: Rect,
    pub scale: f64,
}

/// Paints the annotations permanently into the cropped image.
pub fn flatten(image: &mut Image, capture: &Capture) -> Result<(), Box<dyn Error>> {
    if capture.shapes.is_empty() {
        return Ok(());
    }
    let (width, height) = (image.width as usize, image.height as usize);
    let mut surface = cairo::ImageSurface::create(cairo::Format::Rgb24, width as i32, height as i32)?;
    let stride = surface.stride() as usize;
    {
        let mut data = surface.data()?;
        for y in 0..height {
            for x in 0..width {
                let (src, dst) = ((y * width + x) * 3, y * stride + x * 4);
                data[dst] = image.rgb[src + 2];
                data[dst + 1] = image.rgb[src + 1];
                data[dst + 2] = image.rgb[src];
            }
        }
    }
    {
        let cr = cairo::Context::new(&surface)?;
        cr.translate(-f64::from(capture.region.x), -f64::from(capture.region.y));
        cr.scale(capture.scale, capture.scale);
        cr.translate(-f64::from(capture.bounds.x), -f64::from(capture.bounds.y));
        for shape in &capture.shapes {
            draw_shape(&cr, shape, false)?;
        }
    }
    surface.flush();
    let data = surface.data()?;
    for y in 0..height {
        for x in 0..width {
            let (dst, src) = ((y * width + x) * 3, y * stride + x * 4);
            image.rgb[dst] = data[src + 2];
            image.rgb[dst + 1] = data[src + 1];
            image.rgb[dst + 2] = data[src];
        }
    }
    Ok(())
}

/// What the toolbar should show.
pub struct BarState {
    pub tool: Tool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub settings_open: bool,
    pub hover: Option<Hover>,
    pub after_select: AfterSelect,
}

fn fill_rect(cr: &cairo::Context, r: &Rect, hex: u32, alpha: f64) -> Drawn {
    set_color(cr, hex, alpha);
    cr.rectangle(f64::from(r.x), f64::from(r.y), f64::from(r.w), f64::from(r.h));
    cr.fill()
}

/// A panel with a one-pixel accent line just inside its edge.
fn draw_chrome(cr: &cairo::Context, r: &Rect) -> Drawn {
    fill_rect(cr, r, SURFACE, 1.0)?;
    set_color(cr, ACCENT, 1.0);
    cr.set_line_width(1.0);
    cr.rectangle(
        f64::from(r.x) + 0.5,
        f64::from(r.y) + 0.5,
        f64::from(r.w) - 1.0,
        f64::from(r.h) - 1.0,
    );
    cr.stroke()
}

/// Draws an icon centered on the origin, within about 9 units of it.
fn trace_icon(cr: &cairo::Context, button: Button) -> Drawn {
    let segment = |x0: f64, y0: f64, x1: f64, y1: f64| {
        cr.move_to(x0, y0);
        cr.line_to(x1, y1);
    };
    match button {
        Button::Tool(Tool::Select) => {
            for (i, (x, y)) in [(-6.0, -9.0), (6.0, 2.0), (1.0, 3.0), (4.0, 9.0), (1.0, 10.0), (-2.0, 4.0), (-6.0, 8.0)]
                .into_iter()
                .enumerate()
            {
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.close_path();
        }
        Button::Tool(Tool::Arrow) => {
            segment(-8.0, 8.0, 7.0, -7.0);
            segment(7.0, -7.0, -1.0, -7.0);
            segment(7.0, -7.0, 7.0, 1.0);
        }
        Button::Tool(Tool::Rectangle) => cr.rectangle(-8.0, -6.0, 16.0, 12.0),
        Button::Tool(Tool::Freehand) => {
            cr.move_to(-9.0, 4.0);
            cr.curve_to(-5.0, -12.0, 1.0, 13.0, 9.0, -5.0);
        }
        Button::Tool(Tool::Highlighter) => {
            cr.save()?;
            cr.set_line_width(8.0);
            cr.set_line_cap(cairo::LineCap::Butt);
            segment(-9.0, 2.0, 9.0, 2.0);
            cr.stroke()?;
            cr.restore()?;
            segment(-9.0, -6.0, 9.0, -6.0);
        }
        Button::Tool(Tool::Text) => {
            segment(-7.0, -8.0, 7.0, -8.0);
            segment(0.0, -8.0, 0.0, 9.0);
        }
        Button::Undo | Button::Redo => {
            cr.save()?;
            if button == Button::Redo {
                cr.scale(-1.0, 1.0);
            }
            cr.arc(0.0, 2.0, 7.0, -PI * 0.95, PI * 0.45);
            cr.stroke()?;
            segment(-7.0, 2.0, -9.0, -5.0);
            segment(-7.0, 2.0, -1.0, -2.0);
            cr.stroke()?;
            cr.restore()?;
        }
        Button::Settings => {
            for (y, knob) in [(-6.0, 3.0), (0.0, -3.0), (6.0, 2.0)] {
                segment(-9.0, y, 9.0, y);
                cr.stroke()?;
                cr.arc(knob, y, 2.6, 0.0, TAU);
                cr.fill()?;
            }
        }
    }
    cr.stroke()
}

/// The font of toolbar text, for measuring it before drawing.
pub fn label_font() -> pango::FontDescription {
    font(LABEL_SIZE, false)
}

fn draw_label(cr: &cairo::Context, text: &str, x: f64, center_y: f64, hex: u32, alpha: f64) -> Drawn {
    let layout = pangocairo::functions::create_layout(cr);
    layout.set_font_description(Some(&label_font()));
    layout.set_text(text);
    let (_, height) = layout.pixel_size();
    set_color(cr, hex, alpha);
    cr.move_to(x, center_y - f64::from(height) / 2.0);
    pangocairo::functions::show_layout(cr, &layout);
    Ok(())
}

/// Draws the toolbar in global logical coordinates.
pub fn draw_toolbar(cr: &cairo::Context, bar: &Toolbar, state: &BarState) -> Drawn {
    draw_chrome(cr, &bar.rect)?;
    for (button, r) in &bar.buttons {
        let active = match button {
            Button::Tool(tool) => *tool == state.tool,
            Button::Settings => state.settings_open,
            _ => false,
        };
        let enabled = match button {
            Button::Undo => state.can_undo,
            Button::Redo => state.can_redo,
            _ => true,
        };
        if active {
            fill_rect(cr, r, ACCENT, 1.0)?;
        } else if enabled && state.hover == Some(Hover::Button(*button)) {
            fill_rect(cr, r, TEXT, HOVER_ALPHA)?;
        }
        cr.save()?;
        cr.translate(f64::from(r.x) + f64::from(r.w) / 2.0, f64::from(r.y) + f64::from(r.h) / 2.0);
        cr.set_line_width(ICON_STROKE);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        set_color(cr, if active { SURFACE } else { TEXT }, if enabled { 1.0 } else { DISABLED_ALPHA });
        trace_icon(cr, *button)?;
        cr.restore()?;
    }
    Ok(())
}

/// Draws a tooltip in global logical coordinates.
pub fn draw_tooltip(cr: &cairo::Context, rect: &Rect, text: &str) -> Drawn {
    draw_chrome(cr, rect)?;
    let x = f64::from(rect.x + TOOLTIP_PAD_X);
    let center_y = f64::from(rect.y) + f64::from(rect.h) / 2.0;
    draw_label(cr, text, x, center_y, TEXT, 1.0)
}

/// Draws the settings panel in global logical coordinates.
pub fn draw_settings(cr: &cairo::Context, panel: &SettingsPanel, state: &BarState) -> Drawn {
    const INDENT: f64 = 10.0;
    const RADIO: f64 = 7.0;
    draw_chrome(cr, &panel.rect)?;
    let first_row = panel.options[0].2;
    draw_label(
        cr,
        SettingsPanel::TITLE,
        f64::from(first_row.x) + INDENT,
        f64::from(first_row.y) - 14.0,
        TEXT,
        0.65,
    )?;
    for (option, label, r) in &panel.options {
        if state.hover == Some(Hover::Option(*option)) {
            fill_rect(cr, r, TEXT, HOVER_ALPHA)?;
        }
        let center_y = f64::from(r.y) + f64::from(r.h) / 2.0;
        let radio_x = f64::from(r.x) + INDENT + RADIO;
        set_color(cr, ACCENT, 1.0);
        cr.set_line_width(1.8);
        // Drawing the previous label left a current point the arc would join.
        cr.new_path();
        cr.arc(radio_x, center_y, RADIO, 0.0, TAU);
        cr.stroke()?;
        if *option == state.after_select {
            cr.arc(radio_x, center_y, RADIO - 3.5, 0.0, TAU);
            cr.fill()?;
        }
        draw_label(cr, label, radio_x + RADIO + INDENT, center_y, TEXT, 1.0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn black(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgb: vec![0; (width * height * 3) as usize],
        }
    }

    fn pixel(image: &Image, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * image.width + x) * 3) as usize;
        [image.rgb[i], image.rgb[i + 1], image.rgb[i + 2]]
    }

    const CYAN: [u8; 3] = [0x4D, 0xD8, 0xFF];

    fn capture(shapes: Vec<Shape>, region: Rect, bounds: Rect, scale: f64) -> Capture {
        Capture { region, shapes, bounds, scale }
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn nothing_to_draw_leaves_the_image_alone() {
        let mut image = black(10, 10);
        flatten(&mut image, &capture(vec![], rect(0, 0, 10, 10), rect(0, 0, 10, 10), 1.0)).unwrap();
        assert_eq!(image, black(10, 10));
    }

    #[test]
    fn annotations_land_at_their_place_in_the_crop() {
        // Layout starts at -1440, the crop at frame pixel 1540, which is logical 100.
        let shapes = vec![Shape::Rectangle { a: (110, 20), b: (150, 60) }];
        let mut image = black(100, 100);
        flatten(&mut image, &capture(shapes, rect(1540, 0, 100, 100), rect(-1440, 0, 6800, 2560), 1.0)).unwrap();
        assert_eq!(pixel(&image, 10, 40), CYAN);
        assert_eq!(pixel(&image, 30, 20), CYAN);
        assert_eq!(pixel(&image, 30, 40), [0, 0, 0]);
    }

    #[test]
    fn annotations_follow_the_image_scale() {
        let shapes = vec![Shape::Rectangle { a: (10, 10), b: (30, 30) }];
        let mut image = black(100, 100);
        flatten(&mut image, &capture(shapes, rect(0, 0, 100, 100), rect(0, 0, 50, 50), 2.0)).unwrap();
        assert_eq!(pixel(&image, 20, 40), CYAN);
        assert_eq!(pixel(&image, 10, 40), [0, 0, 0]);
    }

    #[test]
    fn highlighter_is_translucent() {
        let shapes = vec![Shape::Highlighter(vec![(5, 50), (95, 50)])];
        let mut image = black(100, 100);
        flatten(&mut image, &capture(shapes, rect(0, 0, 100, 100), rect(0, 0, 100, 100), 1.0)).unwrap();
        let [r, g, b] = pixel(&image, 50, 50);
        assert!(r > 0 && r < CYAN[0] && g > 0 && g < CYAN[1] && b > 0 && b < CYAN[2]);
    }

    #[test]
    fn every_shape_marks_the_image() {
        let shapes = vec![
            Shape::Arrow { from: (10, 10), to: (90, 10) },
            Shape::Freehand(vec![(10, 30), (90, 30)]),
            Shape::Text { at: (10, 50), text: "Hi".into() },
        ];
        for shape in shapes {
            let mut image = black(100, 100);
            flatten(&mut image, &capture(vec![shape], rect(0, 0, 100, 100), rect(0, 0, 100, 100), 1.0)).unwrap();
            assert_ne!(image, black(100, 100));
        }
    }
}
