//! Rectangles in the compositor's global logical coordinate space.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
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
