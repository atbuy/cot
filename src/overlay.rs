//! Fullscreen layer surfaces, one per monitor, that show the frozen frame and
//! let the user pick a region of it.

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

use gtk::gdk::{self, RGBA};
use gtk::gio::{self, prelude::*};
use gtk::glib::{self, subclass::prelude::*};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{graphene, EventControllerKey, EventControllerLegacy};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::geometry::{to_image_pixels, Rect};
use crate::image::Image;
use crate::selection::Selector;

const fn rgb(hex: u32) -> RGBA {
    let [_, r, g, b] = hex.to_be_bytes();
    RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

const DIM: RGBA = RGBA::new(0.0, 0.0, 0.0, 0.4);
const ACCENT: RGBA = rgb(0xFF4D6D);
const SURFACE: RGBA = rgb(0x1C1216);
const TEXT: RGBA = rgb(0xFFECEF);
const BORDER: f32 = 2.0;
const HANDLE: f32 = 10.0;
const HANDLE_CORE: f32 = 6.0;

/// State shared by the windows of all monitors.
pub struct Session {
    app: gtk::Application,
    bounds: Rect,
    views: RefCell<Vec<FrozenView>>,
    selector: RefCell<Selector>,
    cursor: Cell<&'static str>,
    result: Cell<Option<Rect>>,
}

impl Session {
    fn selection(&self) -> Option<Rect> {
        self.selector.borrow().selection()
    }

    fn redraw(&self) {
        for view in self.views.borrow().iter() {
            view.queue_draw();
        }
    }

    fn update_cursor(&self) {
        let name = self.selector.borrow().cursor();
        if self.cursor.replace(name) != name {
            for view in self.views.borrow().iter() {
                view.set_cursor_from_name(Some(name));
            }
        }
    }

    fn refresh(&self) {
        self.redraw();
        self.update_cursor();
    }

    fn finish(&self, result: Option<Rect>) {
        self.result.set(result);
        self.app.quit();
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FrozenView {
        pub texture: OnceCell<gdk::Texture>,
        pub monitor: OnceCell<Rect>,
        pub session: OnceCell<Rc<Session>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FrozenView {
        const NAME: &'static str = "CotFrozenView";
        type Type = super::FrozenView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for FrozenView {}

    impl WidgetImpl for FrozenView {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let (Some(texture), Some(monitor), Some(session)) =
                (self.texture.get(), self.monitor.get(), self.session.get())
            else {
                return;
            };
            let widget = self.obj();
            let (width, height) = (widget.width() as f32, widget.height() as f32);
            let whole = graphene::Rect::new(0.0, 0.0, width, height);
            snapshot.append_texture(texture, &whole);

            let selection = session.selection();
            let Some(clipped) = selection.and_then(|s| s.intersect(monitor)) else {
                snapshot.append_color(&DIM, &whole);
                return;
            };

            // Dim the four bands around the part of the selection on this monitor.
            let left = (clipped.x - monitor.x) as f32;
            let top = (clipped.y - monitor.y) as f32;
            let right = left + clipped.w as f32;
            let bottom = top + clipped.h as f32;
            for (x, y, w, h) in [
                (0.0, 0.0, width, top),
                (0.0, bottom, width, height - bottom),
                (0.0, top, left, bottom - top),
                (right, top, width - right, bottom - top),
            ] {
                snapshot.append_color(&DIM, &graphene::Rect::new(x, y, w, h));
            }

            // Unclipped, so no edge is drawn where a selection continues on
            // the next monitor. An edge flush with the screen is drawn inside,
            // as there is nowhere outside it to draw.
            let Some(sel) = selection else { return };
            let x = (sel.x - monitor.x) as f32;
            let y = (sel.y - monitor.y) as f32;
            let (w, h) = (sel.w as f32, sel.h as f32);
            let outer_left = if sel.x == monitor.x { x } else { x - BORDER };
            let outer_right = if sel.right() == monitor.right() { x + w } else { x + w + BORDER };
            let outer_top = if sel.y == monitor.y { y } else { y - BORDER };
            let outer_bottom = if sel.bottom() == monitor.bottom() { y + h } else { y + h + BORDER };
            let (outer_w, outer_h) = (outer_right - outer_left, outer_bottom - outer_top);
            for (bx, by, bw, bh) in [
                (outer_left, outer_top, outer_w, BORDER),
                (outer_left, outer_bottom - BORDER, outer_w, BORDER),
                (outer_left, outer_top, BORDER, outer_h),
                (outer_right - BORDER, outer_top, BORDER, outer_h),
            ] {
                snapshot.append_color(&ACCENT, &graphene::Rect::new(bx, by, bw, bh));
            }

            if session.selector.borrow().shows_handles() {
                draw_handles(snapshot, (x, y, w, h), (width, height));
            }

            if monitor.contains((sel.x, sel.y)) {
                self.draw_size_label(snapshot, &sel, session, x, y);
            }
        }
    }

    impl FrozenView {
        /// Shows the size in output pixels, so it matches the saved image.
        fn draw_size_label(&self, snapshot: &gtk::Snapshot, sel: &Rect, session: &Session, x: f32, y: f32) {
            const PAD: f32 = 6.0;
            let image_width = (session.bounds.w as f64 * self.scale()).round() as u32;
            let px = to_image_pixels(sel, &session.bounds, image_width);
            let layout = self.obj().create_pango_layout(Some(&format!("{} × {}", px.w, px.h)));
            let (text_w, text_h) = layout.pixel_size();
            let (box_w, box_h) = (text_w as f32 + 2.0 * PAD, text_h as f32 + PAD);

            let above = y - BORDER - box_h - 4.0;
            let box_y = if above >= 0.0 { above } else { y + 4.0 };
            let box_x = x.max(4.0);

            snapshot.append_color(&SURFACE, &graphene::Rect::new(box_x, box_y, box_w, box_h));
            snapshot.save();
            snapshot.translate(&graphene::Point::new(box_x + PAD, box_y + PAD / 2.0));
            snapshot.append_layout(&layout, &TEXT);
            snapshot.restore();
        }

        fn scale(&self) -> f64 {
            match (self.texture.get(), self.monitor.get()) {
                (Some(texture), Some(monitor)) => texture.width() as f64 / monitor.w as f64,
                _ => 1.0,
            }
        }
    }
}

/// Draws the eight handles, keeping those on a screen edge fully visible.
fn draw_handles(snapshot: &gtk::Snapshot, (x, y, w, h): (f32, f32, f32, f32), (max_x, max_y): (f32, f32)) {
    let inset = |c: f32, max: f32| {
        if c == 0.0 {
            HANDLE / 2.0
        } else if c == max {
            max - HANDLE / 2.0
        } else {
            c
        }
    };
    let square = |cx: f32, cy: f32, size: f32| {
        graphene::Rect::new(cx - size / 2.0, cy - size / 2.0, size, size)
    };
    for hx in [x, x + w / 2.0, x + w] {
        for hy in [y, y + h / 2.0, y + h] {
            if hx == x + w / 2.0 && hy == y + h / 2.0 {
                continue;
            }
            let (cx, cy) = (inset(hx, max_x), inset(hy, max_y));
            snapshot.append_color(&ACCENT, &square(cx, cy, HANDLE));
            snapshot.append_color(&TEXT, &square(cx, cy, HANDLE_CORE));
        }
    }
}

glib::wrapper! {
    pub struct FrozenView(ObjectSubclass<imp::FrozenView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

/// Shows the frozen frame and blocks until the user has chosen. Returns the
/// region in image pixels, or `None` if they cancelled.
pub fn select_region(frame: Rc<Image>) -> Result<Option<Rect>, String> {
    gtk::init().map_err(|e| format!("could not initialize GTK: {e}"))?;
    let display = gdk::Display::default().ok_or("no display")?;
    let monitors: Vec<gdk::Monitor> = display
        .monitors()
        .iter::<gdk::Monitor>()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("could not list monitors: {e}"))?;
    let geometry = |m: &gdk::Monitor| {
        let g = m.geometry();
        Rect {
            x: g.x(),
            y: g.y(),
            w: g.width(),
            h: g.height(),
        }
    };
    let rects: Vec<Rect> = monitors.iter().map(geometry).collect();
    let bounds = rects
        .iter()
        .copied()
        .reduce(|a, b| a.union(&b))
        .ok_or("no monitors found")?;

    let app = gtk::Application::new(Some("dev.atbuy.cot"), gio::ApplicationFlags::NON_UNIQUE);
    let session = Rc::new(Session {
        app: app.clone(),
        bounds,
        views: RefCell::default(),
        selector: RefCell::new(Selector::new(rects, bounds)),
        cursor: Cell::new("crosshair"),
        result: Cell::new(None),
    });

    {
        let session = session.clone();
        let frame = frame.clone();
        app.connect_activate(move |app| {
            for monitor in &monitors {
                if let Some(window) = build_window(app, monitor, geometry(monitor), &frame, &session) {
                    window.present();
                }
            }
        });
    }
    // Our arguments were already handled by clap; GTK must not see them.
    app.run_with_args::<&str>(&[]);

    let scale_base = frame.width;
    Ok(session
        .result
        .get()
        .map(|rect| to_image_pixels(&rect, &bounds, scale_base)))
}

fn build_window(
    app: &gtk::Application,
    monitor: &gdk::Monitor,
    rect: Rect,
    frame: &Image,
    session: &Rc<Session>,
) -> Option<gtk::ApplicationWindow> {
    let pixels = frame.crop(&to_image_pixels(&rect, &session.bounds, frame.width))?;
    let stride = pixels.width as usize * 3;
    let texture = gdk::MemoryTexture::new(
        pixels.width as i32,
        pixels.height as i32,
        gdk::MemoryFormat::R8g8b8,
        &glib::Bytes::from_owned(pixels.rgb),
        stride,
    );

    let view: FrozenView = glib::Object::new();
    let _ = view.imp().texture.set(texture.upcast());
    let _ = view.imp().monitor.set(rect);
    let _ = view.imp().session.set(session.clone());
    session.views.borrow_mut().push(view.clone());

    let window = gtk::ApplicationWindow::new(app);
    window.init_layer_shell();
    window.set_namespace(Some("cot"));
    window.set_layer(Layer::Overlay);
    window.set_monitor(Some(monitor));
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(-1);
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
    window.set_cursor_from_name(Some("crosshair"));
    window.set_child(Some(&view));

    window.add_controller(pointer_controller(rect, session.clone()));
    window.add_controller(key_controller(session.clone()));
    Some(window)
}

/// Uses raw events instead of a drag gesture because the button may be
/// released over a different monitor's window than the one it was pressed on.
fn pointer_controller(monitor: Rect, session: Rc<Session>) -> EventControllerLegacy {
    let controller = EventControllerLegacy::new();
    controller.connect_event(move |_, event| {
        if let Some((x, y)) = event.position() {
            let global = (monitor.x + x.round() as i32, monitor.y + y.round() as i32);
            session.selector.borrow_mut().set_pointer(global);
        }
        let button = event.downcast_ref::<gdk::ButtonEvent>().map(|b| b.button());
        match (event.event_type(), button) {
            (gdk::EventType::ButtonPress, Some(gdk::BUTTON_PRIMARY)) => {
                session.selector.borrow_mut().press();
                session.refresh();
            }
            (gdk::EventType::ButtonPress, Some(gdk::BUTTON_SECONDARY)) => session.finish(None),
            (gdk::EventType::ButtonRelease, Some(gdk::BUTTON_PRIMARY)) => {
                session.selector.borrow_mut().release();
                session.refresh();
            }
            (gdk::EventType::MotionNotify, _) => {
                if session.selector.borrow().is_dragging() {
                    session.redraw();
                }
                session.update_cursor();
            }
            _ => {}
        }
        glib::Propagation::Stop
    });
    controller
}

fn key_controller(session: Rc<Session>) -> EventControllerKey {
    let controller = EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| match key {
        gdk::Key::Escape => {
            session.finish(None);
            glib::Propagation::Stop
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            session.finish(Some(session.selector.borrow().confirmed()));
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    controller
}
