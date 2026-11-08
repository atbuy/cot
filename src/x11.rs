//! Everything cot asks of an X server: the frozen frame, the window list, and
//! overlay windows that the window manager leaves alone.

use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::CURRENT_TIME;

use crate::geometry::Rect;
use crate::image::Image;

/// The part of the root window covered by monitors. The root can be larger
/// than that, and the overlay's coordinates only describe the monitors.
fn monitor_area(conn: &RustConnection, screen: &Screen) -> Rect {
    let root = Rect { x: 0, y: 0, w: i32::from(screen.width_in_pixels), h: i32::from(screen.height_in_pixels) };
    let monitors = conn
        .randr_get_monitors(screen.root, true)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.monitors)
        .unwrap_or_default();
    monitors
        .iter()
        .map(|m| Rect { x: i32::from(m.x), y: i32::from(m.y), w: i32::from(m.width), h: i32::from(m.height) })
        .reduce(|a, b| a.union(&b))
        .and_then(|area| area.intersect(&root))
        .unwrap_or(root)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PixelLayout {
    red: u32,
    green: u32,
    blue: u32,
    big_endian: bool,
}

/// Reduces one masked channel to 8 bits, for depths other than 24 as well.
fn channel(pixel: u32, mask: u32) -> u8 {
    let value = (pixel & mask) >> mask.trailing_zeros();
    match mask.count_ones() {
        bits @ 8.. => (value >> (bits - 8)) as u8,
        bits => (value << (8 - bits)) as u8,
    }
}

/// Converts a 32 bits per pixel Z-format image to RGB.
fn decode(data: &[u8], width: u32, height: u32, layout: PixelLayout) -> Result<Image, String> {
    let expected = width as usize * height as usize * 4;
    if data.len() != expected {
        return Err(format!("expected {expected} bytes from the X server, got {}", data.len()));
    }
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for bytes in data.chunks_exact(4) {
        let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let pixel = if layout.big_endian { u32::from_be_bytes(bytes) } else { u32::from_le_bytes(bytes) };
        rgb.extend_from_slice(&[channel(pixel, layout.red), channel(pixel, layout.green), channel(pixel, layout.blue)]);
    }
    Ok(Image { width, height, rgb })
}

fn pixel_layout(setup: &Setup, screen: &Screen, depth: u8, visual: Visualid) -> Result<PixelLayout, String> {
    let bits = setup.pixmap_formats.iter().find(|f| f.depth == depth).map(|f| f.bits_per_pixel);
    if bits != Some(32) {
        return Err(format!("unsupported X server pixel format: depth {depth}, {bits:?} bits per pixel"));
    }
    // A window's image can name no visual of its own, meaning the root's.
    let wanted = if visual == 0 { screen.root_visual } else { visual };
    let found = screen
        .allowed_depths
        .iter()
        .flat_map(|d| &d.visuals)
        .find(|v| v.visual_id == wanted)
        .ok_or("the X server did not describe the screen's pixel format")?;
    if found.red_mask == 0 || found.green_mask == 0 || found.blue_mask == 0 {
        return Err("cot needs a true color screen".into());
    }
    Ok(PixelLayout {
        red: found.red_mask,
        green: found.green_mask,
        blue: found.blue_mask,
        big_endian: setup.image_byte_order == ImageOrder::MSB_FIRST,
    })
}

/// Blends a premultiplied ARGB cursor over the image. `at` is where the
/// cursor's top-left corner lands, in image pixels.
fn blend_cursor(image: &mut Image, cursor: &[u32], size: (u32, u32), at: (i32, i32)) {
    for row in 0..size.1 as i32 {
        for col in 0..size.0 as i32 {
            let (x, y) = (at.0 + col, at.1 + row);
            if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
                continue;
            }
            let [a, r, g, b] = cursor[(row as usize) * size.0 as usize + col as usize].to_be_bytes();
            let i = (y as usize * image.width as usize + x as usize) * 3;
            for (c, source) in [r, g, b].into_iter().enumerate() {
                let under = u32::from(image.rgb[i + c]) * (255 - u32::from(a)) / 255;
                image.rgb[i + c] = (u32::from(source) + under).min(255) as u8;
            }
        }
    }
}

fn draw_cursor(conn: &RustConnection, image: &mut Image, area: &Rect) -> Result<(), Box<dyn Error>> {
    // XFixes answers nothing until a client has said which version it speaks.
    conn.xfixes_query_version(4, 0)?.reply()?;
    let cursor = conn.xfixes_get_cursor_image()?.reply()?;
    let at = (
        i32::from(cursor.x) - i32::from(cursor.xhot) - area.x,
        i32::from(cursor.y) - i32::from(cursor.yhot) - area.y,
    );
    blend_cursor(image, &cursor.cursor_image, (u32::from(cursor.width), u32::from(cursor.height)), at);
    Ok(())
}

/// Reads every monitor at once, before any window of ours exists.
pub fn capture(include_cursor: bool) -> Result<Image, Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| format!("could not connect to the X server: {e}"))?;
    let screen = &conn.setup().roots[screen_num];
    let area = monitor_area(&conn, screen);
    let reply = conn
        .get_image(ImageFormat::Z_PIXMAP, screen.root, area.x as i16, area.y as i16, area.w as u16, area.h as u16, !0)?
        .reply()
        .map_err(|e| format!("could not read the screen: {e}"))?;
    let layout = pixel_layout(conn.setup(), screen, reply.depth, reply.visual)?;
    let mut image = decode(&reply.data, area.w as u32, area.h as u32, layout)?;
    if include_cursor {
        if let Err(e) = draw_cursor(&conn, &mut image, &area) {
            eprintln!("cot: leaving the cursor out: {e}");
        }
    }
    Ok(image)
}

#[derive(Clone, Copy, Debug)]
struct Client {
    rect: Rect,
    viewable: bool,
    hidden: bool,
    dock: bool,
}

/// Picks what can be snapped to from a bottom-to-top list, and returns it
/// topmost first in the overlay's coordinates. Only physical pixels reach
/// here, so a scaled display needs them divided.
fn topmost_first(bottom_to_top: Vec<Client>, scale: f64) -> Vec<Rect> {
    let logical = |v: i32| (f64::from(v) / scale).round() as i32;
    bottom_to_top
        .into_iter()
        .rev()
        .filter(|c| c.viewable && !c.hidden && !c.dock && c.rect.w > 0 && c.rect.h > 0)
        .map(|c| Rect { x: logical(c.rect.x), y: logical(c.rect.y), w: logical(c.rect.w), h: logical(c.rect.h) })
        .collect()
}

struct Atoms {
    state: Atom,
    hidden: Atom,
    kind: Atom,
    dock: Atom,
}

fn atom(conn: &RustConnection, name: &str) -> Result<Atom, Box<dyn Error>> {
    Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
}

fn atoms_of(conn: &RustConnection, window: Window, property: Atom) -> Vec<Atom> {
    conn.get_property(false, window, property, AtomEnum::ATOM, 0, 64)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32().map(Iterator::collect))
        .unwrap_or_default()
}

/// `None` for a window that closed while we were asking about it.
fn describe(conn: &RustConnection, root: Window, window: Window, atoms: &Atoms) -> Option<Client> {
    let attributes = conn.get_window_attributes(window).ok()?.reply().ok()?;
    let geometry = conn.get_geometry(window).ok()?.reply().ok()?;
    let origin = conn.translate_coordinates(window, root, 0, 0).ok()?.reply().ok()?;
    Some(Client {
        rect: Rect {
            x: i32::from(origin.dst_x),
            y: i32::from(origin.dst_y),
            w: i32::from(geometry.width),
            h: i32::from(geometry.height),
        },
        // Viewable also needs every ancestor mapped, which is how i3 hides
        // the windows of other workspaces.
        viewable: attributes.map_state == MapState::VIEWABLE,
        hidden: atoms_of(conn, window, atoms.state).contains(&atoms.hidden),
        dock: atoms_of(conn, window, atoms.kind).contains(&atoms.dock),
    })
}

/// The windows on screen, topmost first, from the standard EWMH list.
pub fn window_stack(scale: f64) -> Result<Vec<Rect>, Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;
    let list = atom(&conn, "_NET_CLIENT_LIST_STACKING")?;
    let windows: Vec<Window> = conn
        .get_property(false, root, list, AtomEnum::WINDOW, 0, u32::MAX / 4)?
        .reply()?
        .value32()
        .ok_or("the window manager does not publish a window list")?
        .collect();
    let atoms = Atoms {
        state: atom(&conn, "_NET_WM_STATE")?,
        hidden: atom(&conn, "_NET_WM_STATE_HIDDEN")?,
        kind: atom(&conn, "_NET_WM_WINDOW_TYPE")?,
        dock: atom(&conn, "_NET_WM_WINDOW_TYPE_DOCK")?,
    };
    let clients = windows.into_iter().filter_map(|w| describe(&conn, root, w, &atoms)).collect();
    Ok(topmost_first(clients, scale))
}

/// A connection for placing overlay windows, which GTK can't do on X11.
pub struct Overlay {
    conn: RustConnection,
}

impl Overlay {
    pub fn connect() -> Result<Overlay, String> {
        let (conn, _) = x11rb::connect(None).map_err(|e| format!("could not connect to the X server: {e}"))?;
        Ok(Overlay { conn })
    }

    fn sync(&self) -> Result<(), Box<dyn Error>> {
        self.conn.get_input_focus()?.reply()?;
        Ok(())
    }

    /// Hides a window from the window manager and pins it to a monitor. Must
    /// come before the window is mapped, or a tiling manager will tile it.
    pub fn make_unmanaged(&self, window: Window, rect: &Rect) -> Result<(), Box<dyn Error>> {
        self.conn.change_window_attributes(window, &ChangeWindowAttributesAux::new().override_redirect(1))?;
        self.conn.configure_window(
            window,
            &ConfigureWindowAux::new().x(rect.x).y(rect.y).width(rect.w as u32).height(rect.h as u32),
        )?;
        self.sync()
    }

    /// Unmanaged windows never receive focus from the window manager, so it
    /// has to be taken, and taken back if a new window claims it. Asking
    /// before the window is viewable would be refused, so that case waits.
    pub fn hold_focus(&self, window: Window) -> Result<(), Box<dyn Error>> {
        if self.conn.get_input_focus()?.reply()?.focus == window
            || self.conn.get_window_attributes(window)?.reply()?.map_state != MapState::VIEWABLE
        {
            return Ok(());
        }
        self.conn.configure_window(window, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))?;
        self.conn.set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?;
        self.sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BGRX: PixelLayout = PixelLayout { red: 0xFF0000, green: 0xFF00, blue: 0xFF, big_endian: false };

    fn client(x: i32) -> Client {
        Client { rect: Rect { x, y: 0, w: 100, h: 100 }, viewable: true, hidden: false, dock: false }
    }

    #[test]
    fn decodes_the_usual_little_endian_layout() {
        let image = decode(&[3, 2, 1, 0, 30, 20, 10, 0], 2, 1, BGRX).unwrap();
        assert_eq!(image.rgb, [1, 2, 3, 10, 20, 30]);
    }

    #[test]
    fn decodes_big_endian_servers() {
        let layout = PixelLayout { big_endian: true, ..BGRX };
        assert_eq!(decode(&[0, 1, 2, 3], 1, 1, layout).unwrap().rgb, [1, 2, 3]);
    }

    #[test]
    fn decodes_ten_bit_channels_to_eight() {
        let layout = PixelLayout { red: 0x3FF00000, green: 0xFFC00, blue: 0x3FF, big_endian: false };
        let pixel: u32 = (0x3FF << 20) | (0x200 << 10);
        assert_eq!(decode(&pixel.to_le_bytes(), 1, 1, layout).unwrap().rgb, [0xFF, 0x80, 0]);
    }

    #[test]
    fn refuses_a_short_reply() {
        assert!(decode(&[0; 7], 2, 1, BGRX).is_err());
    }

    #[test]
    fn the_top_of_the_stack_comes_first() {
        let xs: Vec<i32> = topmost_first(vec![client(1), client(2)], 1.0).iter().map(|r| r.x).collect();
        assert_eq!(xs, [2, 1]);
    }

    #[test]
    fn leaves_out_what_cannot_be_seen() {
        let unmapped = Client { viewable: false, ..client(1) };
        let hidden = Client { hidden: true, ..client(2) };
        let dock = Client { dock: true, ..client(3) };
        let empty = Client { rect: Rect { x: 4, y: 0, w: 0, h: 10 }, ..client(4) };
        assert_eq!(topmost_first(vec![unmapped, hidden, dock, empty, client(5)], 1.0).len(), 1);
    }

    #[test]
    fn scales_to_overlay_coordinates() {
        let rects = topmost_first(vec![Client { rect: Rect { x: 200, y: 40, w: 300, h: 100 }, ..client(0) }], 2.0);
        assert_eq!(rects, [Rect { x: 100, y: 20, w: 150, h: 50 }]);
    }

    #[test]
    fn an_opaque_cursor_replaces_and_a_clear_one_keeps() {
        let mut image = Image { width: 2, height: 1, rgb: vec![9; 6] };
        blend_cursor(&mut image, &[0xFF010203, 0x00000000], (2, 1), (0, 0));
        assert_eq!(image.rgb, [1, 2, 3, 9, 9, 9]);
    }

    #[test]
    fn a_cursor_past_the_edge_is_clipped() {
        let mut image = Image { width: 1, height: 1, rgb: vec![9; 3] };
        blend_cursor(&mut image, &[0xFF000000; 4], (2, 2), (-1, -1));
        assert_eq!(image.rgb, [0, 0, 0]);
    }
}
