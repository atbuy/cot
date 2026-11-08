//! Where the magnifier goes and how large it is.

use crate::geometry::Rect;

/// Source pixels shown on each side of the one under the cursor.
pub const RADIUS: i32 = 10;
pub const ZOOM: i32 = 8;
pub const PAD: i32 = 6;
pub const LABEL_HEIGHT: i32 = 24;

const GRID: i32 = (2 * RADIUS + 1) * ZOOM;
const WIDTH: i32 = GRID + 2 * PAD;
const HEIGHT: i32 = GRID + 2 * PAD + LABEL_HEIGHT;
/// Keeps the magnifier off the point being aimed at.
const CURSOR_OFFSET: i32 = 24;
const SCREEN_MARGIN: i32 = 8;

pub struct Magnifier {
    pub rect: Rect,
    /// The pointer, in global logical coordinates.
    pub center: (i32, i32),
}

impl Magnifier {
    /// Sits below and right of the pointer, flipping away from screen edges.
    pub fn new(center: (i32, i32), monitor: &Rect) -> Self {
        let place = |p: i32, size: i32, start: i32, end: i32| {
            let after = p + CURSOR_OFFSET;
            let position = if after + size <= end - SCREEN_MARGIN { after } else { p - CURSOR_OFFSET - size };
            position.max(start + SCREEN_MARGIN)
        };
        Magnifier {
            rect: Rect {
                x: place(center.0, WIDTH, monitor.x, monitor.right()),
                y: place(center.1, HEIGHT, monitor.y, monitor.bottom()),
                w: WIDTH,
                h: HEIGHT,
            },
            center,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Rect = Rect { x: -1440, y: 0, w: 1440, h: 2560 };

    fn inside(outer: &Rect, inner: &Rect) -> bool {
        inner.x >= outer.x && inner.y >= outer.y && inner.right() <= outer.right() && inner.bottom() <= outer.bottom()
    }

    #[test]
    fn sits_below_and_right_of_the_pointer() {
        let m = Magnifier::new((-800, 1000), &MONITOR);
        assert_eq!((m.rect.x, m.rect.y), (-800 + CURSOR_OFFSET, 1000 + CURSOR_OFFSET));
    }

    #[test]
    fn flips_near_the_right_and_bottom_edges() {
        let m = Magnifier::new((-5, 2555), &MONITOR);
        assert!(m.rect.right() < -5);
        assert!(m.rect.bottom() < 2555);
    }

    #[test]
    fn stays_on_the_monitor_and_off_the_pointer_everywhere() {
        for x in (-1440..0).step_by(37) {
            for y in (0..2560).step_by(53) {
                let m = Magnifier::new((x, y), &MONITOR);
                assert!(inside(&MONITOR, &m.rect), "off screen at {x},{y}");
                assert!(!m.rect.contains((x, y)), "covers pointer at {x},{y}");
            }
        }
    }
}
