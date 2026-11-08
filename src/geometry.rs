//! Rectangles in the compositor's global logical coordinate space.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Which edges of a rectangle a resize moves. A corner sets two of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Handle {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Handle(Handle),
    Inside,
    Outside,
}

impl Rect {
    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        Rect {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            w: (a.0 - b.0).abs(),
            h: (a.1 - b.1).abs(),
        }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn contains(&self, p: (i32, i32)) -> bool {
        p.0 >= self.x && p.0 < self.right() && p.1 >= self.y && p.1 < self.bottom()
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > x && bottom > y).then_some(Rect {
            x,
            y,
            w: right - x,
            h: bottom - y,
        })
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect {
            x,
            y,
            w: self.right().max(other.right()) - x,
            h: self.bottom().max(other.bottom()) - y,
        }
    }

    /// Moves the point inside the rectangle, keeping it on a valid pixel.
    pub fn clamp_point(&self, p: (i32, i32)) -> (i32, i32) {
        (
            p.0.clamp(self.x, self.right() - 1),
            p.1.clamp(self.y, self.bottom() - 1),
        )
    }

    /// The cursor can only reach the last pixel of a monitor, one short of its
    /// edge. Pushes that pixel onto the edge so a selection can reach it.
    pub fn snap_far_edges(&self, p: (i32, i32)) -> (i32, i32) {
        (
            if p.0 == self.right() - 1 { self.right() } else { p.0 },
            if p.1 == self.bottom() - 1 { self.bottom() } else { p.1 },
        )
    }

    /// Finds what a press at `p` would grab. `reach` is how far from an edge,
    /// on either side, still counts as grabbing it.
    pub fn hit_test(&self, p: (i32, i32), reach: i32) -> Hit {
        let in_x = p.0 >= self.x - reach && p.0 <= self.right() + reach;
        let in_y = p.1 >= self.y - reach && p.1 <= self.bottom() + reach;
        if !in_x || !in_y {
            return Hit::Outside;
        }
        let (left, right) = ((p.0 - self.x).abs(), (p.0 - self.right()).abs());
        let (top, bottom) = ((p.1 - self.y).abs(), (p.1 - self.bottom()).abs());
        let handle = Handle {
            left: left <= reach && left <= right,
            right: right <= reach && right < left,
            top: top <= reach && top <= bottom,
            bottom: bottom <= reach && bottom < top,
        };
        if handle == Handle::default() {
            Hit::Inside
        } else {
            Hit::Handle(handle)
        }
    }

    /// Moves the handle's edges to `p`. Dragging an edge past the opposite one
    /// flips the rectangle rather than collapsing it.
    pub fn resized(&self, handle: Handle, p: (i32, i32)) -> Rect {
        let x0 = if handle.left { p.0 } else { self.x };
        let x1 = if handle.right { p.0 } else { self.right() };
        let y0 = if handle.top { p.1 } else { self.y };
        let y1 = if handle.bottom { p.1 } else { self.bottom() };
        Rect::from_points((x0, y0), (x1, y1))
    }

    /// Moves the rectangle by the given offset, stopping at the edges of `bounds`.
    pub fn moved_within(&self, dx: i32, dy: i32, bounds: &Rect) -> Rect {
        Rect {
            x: (self.x + dx).min(bounds.right() - self.w).max(bounds.x),
            y: (self.y + dy).min(bounds.bottom() - self.h).max(bounds.y),
            ..*self
        }
    }
}

/// Maps a logical rectangle to pixels of a screenshot covering `bounds`.
///
/// grim renders the whole layout at one scale, so the ratio between the image
/// and the layout is the same everywhere, whatever each monitor's rotation.
pub fn to_image_pixels(rect: &Rect, bounds: &Rect, image_width: u32) -> Rect {
    let scale = f64::from(image_width) / f64::from(bounds.w);
    let scaled = |v: i32| (f64::from(v) * scale).round() as i32;
    let x = scaled(rect.x - bounds.x);
    let y = scaled(rect.y - bounds.y);
    Rect {
        x,
        y,
        w: scaled(rect.right() - bounds.x) - x,
        h: scaled(rect.bottom() - bounds.y) - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn from_points_normalizes_any_drag_direction() {
        assert_eq!(Rect::from_points((30, 40), (10, 5)), rect(10, 5, 20, 35));
    }

    #[test]
    fn union_covers_rotated_monitor_left_of_origin() {
        let rotated = rect(-1440, 0, 1440, 2560);
        let ultrawide = rect(0, 0, 3440, 1440);
        let right = rect(3440, 0, 1920, 1080);
        let all = rotated.union(&ultrawide).union(&right);
        assert_eq!(all, rect(-1440, 0, 6800, 2560));
    }

    #[test]
    fn intersect_is_none_for_adjacent_monitors() {
        assert_eq!(rect(0, 0, 10, 10).intersect(&rect(10, 0, 10, 10)), None);
    }

    #[test]
    fn intersect_clips_selection_to_monitor() {
        let selection = rect(-20, 10, 50, 10);
        assert_eq!(
            selection.intersect(&rect(0, 0, 100, 100)),
            Some(rect(0, 10, 30, 10))
        );
    }

    const BOTTOM_RIGHT: Handle = Handle { left: false, right: true, top: false, bottom: true };
    const LEFT: Handle = Handle { left: true, right: false, top: false, bottom: false };

    #[test]
    fn hit_test_finds_corners_edges_and_interior() {
        let r = rect(100, 100, 200, 200);
        assert_eq!(r.hit_test((298, 303), 8), Hit::Handle(BOTTOM_RIGHT));
        assert_eq!(r.hit_test((95, 200), 8), Hit::Handle(LEFT));
        assert_eq!(r.hit_test((200, 200), 8), Hit::Inside);
        assert_eq!(r.hit_test((50, 200), 8), Hit::Outside);
    }

    #[test]
    fn hit_test_picks_the_nearer_edge_of_a_tiny_selection() {
        let r = rect(100, 100, 4, 100);
        assert_eq!(r.hit_test((103, 150), 8), Hit::Handle(Handle { right: true, ..Handle::default() }));
    }

    #[test]
    fn resize_only_moves_the_grabbed_edges() {
        let r = rect(100, 100, 200, 200);
        assert_eq!(r.resized(LEFT, (50, 999)), rect(50, 100, 250, 200));
        assert_eq!(r.resized(BOTTOM_RIGHT, (500, 450)), rect(100, 100, 400, 350));
    }

    #[test]
    fn resize_past_the_opposite_edge_flips() {
        let r = rect(100, 100, 200, 200);
        assert_eq!(r.resized(LEFT, (400, 150)), rect(300, 100, 100, 200));
    }

    #[test]
    fn move_stops_at_the_layout_bounds() {
        let bounds = rect(-1440, 0, 6800, 2560);
        let r = rect(0, 0, 100, 100);
        assert_eq!(r.moved_within(-9000, -5, &bounds), rect(-1440, 0, 100, 100));
        assert_eq!(r.moved_within(9000, 9000, &bounds), rect(5260, 2460, 100, 100));
    }

    #[test]
    fn last_pixel_snaps_to_the_far_edge() {
        let monitor = rect(0, 0, 3440, 1440);
        assert_eq!(monitor.snap_far_edges((3439, 1439)), (3440, 1440));
        assert_eq!(monitor.snap_far_edges((3438, 0)), (3438, 0));
    }

    #[test]
    fn pixels_are_offset_from_layout_origin() {
        let bounds = rect(-1440, 0, 6800, 2560);
        assert_eq!(
            to_image_pixels(&rect(-1440, 100, 50, 60), &bounds, 6800),
            rect(0, 100, 50, 60)
        );
        assert_eq!(
            to_image_pixels(&rect(3440, 0, 1920, 1080), &bounds, 6800),
            rect(4880, 0, 1920, 1080)
        );
    }

    #[test]
    fn pixels_follow_image_scale() {
        let bounds = rect(0, 0, 100, 100);
        assert_eq!(
            to_image_pixels(&rect(10, 20, 30, 40), &bounds, 200),
            rect(20, 40, 60, 80)
        );
    }
}
