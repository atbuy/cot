//! What the user has selected and how the mouse changes it, in global logical
//! coordinates so a selection can span monitors. Free of GTK so it can be
//! tested without a display.

use crate::geometry::{Handle, Hit, Rect};

/// A press and release closer together than this is a click, not a drag.
const CLICK_SLOP: i32 = 4;
/// How far from a selection edge a press still grabs it.
const HANDLE_REACH: i32 = 8;

type Point = (i32, i32);

#[derive(Clone, Copy)]
enum Mode {
    Idle,
    Drawing { anchor: Point, previous: Option<Rect> },
    Settled(Rect),
    Moving { start: Rect, grab: Point },
    Resizing { start: Rect, handle: Handle },
}

pub struct Selector {
    monitors: Vec<Rect>,
    bounds: Rect,
    pointer: Point,
    mode: Mode,
}

impl Selector {
    pub fn new(monitors: Vec<Rect>, bounds: Rect) -> Self {
        Selector {
            monitors,
            bounds,
            pointer: (bounds.x, bounds.y),
            mode: Mode::Idle,
        }
    }

    pub fn set_pointer(&mut self, p: Point) {
        self.pointer = self.bounds.clamp_point(p);
    }

    pub fn pointer(&self) -> Point {
        self.pointer
    }

    fn monitor_under_pointer(&self) -> Option<Rect> {
        self.monitors.iter().copied().find(|m| m.contains(self.pointer))
    }

    /// Where a selection edge should go for the current pointer position.
    fn pointer_edge(&self) -> Point {
        self.monitor_under_pointer()
            .map_or(self.pointer, |m| m.snap_far_edges(self.pointer))
    }

    pub fn press(&mut self) {
        let draw = |previous| Mode::Drawing {
            anchor: self.pointer_edge(),
            previous,
        };
        self.mode = match self.mode {
            Mode::Idle => draw(None),
            Mode::Settled(rect) => match rect.hit_test(self.pointer, HANDLE_REACH) {
                Hit::Handle(handle) => Mode::Resizing { start: rect, handle },
                Hit::Inside => Mode::Moving { start: rect, grab: self.pointer },
                Hit::Outside => draw(Some(rect)),
            },
            busy => busy,
        };
    }

    /// Ends a drag and leaves the result on screen to be adjusted. A click
    /// selects the monitor under it, unless a selection already exists: that
    /// one is kept, so a stray click can't lose it. Returns whether this
    /// produced a new selection, as opposed to adjusting or keeping one.
    pub fn release(&mut self) -> bool {
        let Some(current) = self.selection() else { return false };
        let (settled, is_new) = match self.mode {
            Mode::Idle | Mode::Settled(_) => return false,
            Mode::Drawing { previous, .. } if current.w < CLICK_SLOP && current.h < CLICK_SLOP => match previous {
                Some(rect) => (rect, false),
                None => (self.monitor_under_pointer().unwrap_or(self.bounds), true),
            },
            Mode::Drawing { .. } => (at_least_one_pixel(current), true),
            _ => (at_least_one_pixel(current), false),
        };
        self.mode = Mode::Settled(settled);
        is_new
    }

    pub fn selection(&self) -> Option<Rect> {
        match self.mode {
            Mode::Idle => None,
            Mode::Drawing { anchor, .. } => Some(Rect::from_points(anchor, self.pointer_edge())),
            Mode::Settled(rect) => Some(rect),
            Mode::Moving { start, grab } => {
                let (dx, dy) = (self.pointer.0 - grab.0, self.pointer.1 - grab.1);
                Some(start.moved_within(dx, dy, &self.bounds))
            }
            Mode::Resizing { start, handle } => Some(start.resized(handle, self.pointer_edge())),
        }
    }

    /// What Enter should take: the selection, or everything if there is none.
    pub fn confirmed(&self) -> Rect {
        self.selection().map_or(self.bounds, at_least_one_pixel)
    }

    pub fn is_dragging(&self) -> bool {
        !matches!(self.mode, Mode::Idle | Mode::Settled(_))
    }

    pub fn shows_handles(&self) -> bool {
        matches!(
            self.mode,
            Mode::Settled(_) | Mode::Moving { .. } | Mode::Resizing { .. }
        )
    }

    /// The CSS cursor name for what a press would do here, or is doing now.
    pub fn cursor(&self) -> &'static str {
        match self.mode {
            Mode::Settled(rect) => match rect.hit_test(self.pointer, HANDLE_REACH) {
                Hit::Handle(handle) => handle_cursor(handle),
                Hit::Inside => "move",
                Hit::Outside => "crosshair",
            },
            Mode::Moving { .. } => "move",
            Mode::Resizing { handle, .. } => handle_cursor(handle),
            Mode::Idle | Mode::Drawing { .. } => "crosshair",
        }
    }
}

fn at_least_one_pixel(rect: Rect) -> Rect {
    Rect {
        w: rect.w.max(1),
        h: rect.h.max(1),
        ..rect
    }
}

fn handle_cursor(handle: Handle) -> &'static str {
    match handle {
        Handle { left: true, top: true, .. } | Handle { right: true, bottom: true, .. } => "nwse-resize",
        Handle { right: true, top: true, .. } | Handle { left: true, bottom: true, .. } => "nesw-resize",
        Handle { top: true, .. } | Handle { bottom: true, .. } => "ns-resize",
        _ => "ew-resize",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The real layout: rotated, ultrawide, then a smaller landscape monitor.
    fn selector() -> Selector {
        let monitors = vec![
            rect(-1440, 0, 1440, 2560),
            rect(0, 0, 3440, 1440),
            rect(3440, 0, 1920, 1080),
        ];
        Selector::new(monitors, rect(-1440, 0, 6800, 2560))
    }

    fn drag(s: &mut Selector, from: Point, to: Point) {
        s.set_pointer(from);
        s.press();
        s.set_pointer(to);
        s.release();
    }

    fn drawn() -> Selector {
        let mut s = selector();
        drag(&mut s, (100, 200), (400, 500));
        s
    }

    #[test]
    fn nothing_selected_at_first_and_enter_takes_everything() {
        let s = selector();
        assert_eq!(s.selection(), None);
        assert_eq!(s.confirmed(), rect(-1440, 0, 6800, 2560));
    }

    #[test]
    fn releasing_a_drag_keeps_the_selection_for_adjusting() {
        let s = drawn();
        assert_eq!(s.selection(), Some(rect(100, 200, 300, 300)));
        assert!(s.shows_handles());
        assert!(!s.is_dragging());
        assert_eq!(s.confirmed(), rect(100, 200, 300, 300));
    }

    #[test]
    fn handles_are_hidden_while_drawing() {
        let mut s = selector();
        s.set_pointer((100, 100));
        s.press();
        s.set_pointer((200, 200));
        assert!(s.is_dragging());
        assert!(!s.shows_handles());
    }

    #[test]
    fn dragging_inside_moves_without_resizing() {
        let mut s = drawn();
        drag(&mut s, (250, 350), (300, 420));
        assert_eq!(s.selection(), Some(rect(150, 270, 300, 300)));
    }

    #[test]
    fn dragging_a_corner_resizes() {
        let mut s = drawn();
        drag(&mut s, (400, 500), (600, 800));
        assert_eq!(s.selection(), Some(rect(100, 200, 500, 600)));
    }

    #[test]
    fn dragging_an_edge_moves_only_that_edge() {
        let mut s = drawn();
        drag(&mut s, (100, 350), (50, 900));
        assert_eq!(s.selection(), Some(rect(50, 200, 350, 300)));
    }

    #[test]
    fn resizing_can_cross_onto_another_monitor() {
        let mut s = drawn();
        drag(&mut s, (100, 350), (-300, 350));
        assert_eq!(s.selection(), Some(rect(-300, 200, 700, 300)));
    }

    #[test]
    fn resizing_to_nothing_leaves_one_pixel() {
        let mut s = drawn();
        drag(&mut s, (100, 350), (400, 350));
        assert_eq!(s.selection(), Some(rect(400, 200, 1, 300)));
    }

    #[test]
    fn dragging_outside_starts_a_new_selection() {
        let mut s = drawn();
        drag(&mut s, (1000, 1000), (1100, 1200));
        assert_eq!(s.selection(), Some(rect(1000, 1000, 100, 200)));
    }

    #[test]
    fn clicking_selects_the_monitor_under_the_pointer() {
        let mut s = selector();
        drag(&mut s, (4000, 500), (4000, 500));
        assert_eq!(s.selection(), Some(rect(3440, 0, 1920, 1080)));
    }

    #[test]
    fn a_stray_click_outside_keeps_the_selection() {
        let mut s = drawn();
        drag(&mut s, (2000, 1000), (2001, 1001));
        assert_eq!(s.selection(), Some(rect(100, 200, 300, 300)));
    }

    #[test]
    fn a_drag_can_reach_the_far_edge_of_a_monitor() {
        let mut s = selector();
        drag(&mut s, (3000, 1000), (3439, 1439));
        assert_eq!(s.selection(), Some(rect(3000, 1000, 440, 440)));
    }

    #[test]
    fn move_stops_at_the_layout_edge() {
        let mut s = drawn();
        drag(&mut s, (250, 350), (250, -500));
        assert_eq!(s.selection(), Some(rect(100, 0, 300, 300)));
    }

    #[test]
    fn release_reports_only_new_selections() {
        let mut s = selector();
        s.set_pointer((100, 200));
        s.press();
        s.set_pointer((400, 500));
        assert!(s.release());
        s.set_pointer((250, 350));
        s.press();
        s.set_pointer((300, 400));
        assert!(!s.release());
        s.set_pointer((2000, 1000));
        s.press();
        assert!(!s.release());
    }

    #[test]
    fn cursor_shows_what_a_press_would_do() {
        let mut s = drawn();
        s.set_pointer((100, 200));
        assert_eq!(s.cursor(), "nwse-resize");
        s.set_pointer((400, 200));
        assert_eq!(s.cursor(), "nesw-resize");
        s.set_pointer((250, 200));
        assert_eq!(s.cursor(), "ns-resize");
        s.set_pointer((400, 350));
        assert_eq!(s.cursor(), "ew-resize");
        s.set_pointer((250, 350));
        assert_eq!(s.cursor(), "move");
        s.set_pointer((1000, 1000));
        assert_eq!(s.cursor(), "crosshair");
    }
}
