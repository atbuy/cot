//! Where the toolbar and its settings panel go, and what is under the pointer.

use crate::editor::Tool;
use crate::geometry::Rect;
use crate::settings::AfterSelect;

const BUTTON: i32 = 36;
const BUTTON_GAP: i32 = 2;
const GROUP_GAP: i32 = 14;
const PAD: i32 = 6;
/// Keeps the toolbar clear of the selection's handles.
const GAP: i32 = 14;
/// Above the selection the toolbar also has to clear the size label.
const GAP_ABOVE: i32 = 44;
const SCREEN_MARGIN: i32 = 8;

const PANEL_WIDTH: i32 = 320;
const PANEL_TITLE: i32 = 30;
const PANEL_ROW: i32 = 38;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Tool(Tool),
    Undo,
    Redo,
    Settings,
}

const GROUPS: [&[Button]; 3] = [
    &[
        Button::Tool(Tool::Select),
        Button::Tool(Tool::Arrow),
        Button::Tool(Tool::Rectangle),
        Button::Tool(Tool::Freehand),
        Button::Tool(Tool::Highlighter),
        Button::Tool(Tool::Text),
    ],
    &[Button::Undo, Button::Redo],
    &[Button::Settings],
];

/// Something clickable, for hover highlighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    Button(Button),
    Option(AfterSelect),
}

pub struct Toolbar {
    pub rect: Rect,
    pub buttons: Vec<(Button, Rect)>,
    monitor: Rect,
}

pub struct SettingsPanel {
    pub rect: Rect,
    pub options: [(AfterSelect, &'static str, Rect); 2],
}

/// Places the toolbar on the monitor holding most of the selection: below the
/// selection if there is room, else above it, else inside its bottom edge.
pub fn place(selection: &Rect, monitors: &[Rect]) -> Option<Toolbar> {
    let area = |r: &Rect| i64::from(r.w) * i64::from(r.h);
    let (monitor, part) = monitors
        .iter()
        .filter_map(|m| selection.intersect(m).map(|part| (*m, part)))
        .max_by_key(|(_, part)| area(part))?;

    let buttons_width: i32 = GROUPS
        .iter()
        .map(|g| g.len() as i32 * BUTTON + (g.len() as i32 - 1) * BUTTON_GAP)
        .sum();
    let w = 2 * PAD + buttons_width + (GROUPS.len() as i32 - 1) * GROUP_GAP;
    let h = 2 * PAD + BUTTON;

    let x = (part.x + part.w / 2 - w / 2)
        .min(monitor.right() - w - SCREEN_MARGIN)
        .max(monitor.x + SCREEN_MARGIN);
    let below = part.bottom() + GAP;
    let above = part.y - GAP_ABOVE - h;
    let y = if below + h <= monitor.bottom() - SCREEN_MARGIN {
        below
    } else if above >= monitor.y + SCREEN_MARGIN {
        above
    } else {
        (part.bottom() - h - GAP).max(monitor.y + SCREEN_MARGIN)
    };

    let mut buttons = Vec::new();
    let mut bx = x + PAD;
    for group in GROUPS {
        for button in group {
            buttons.push((*button, Rect { x: bx, y: y + PAD, w: BUTTON, h: BUTTON }));
            bx += BUTTON + BUTTON_GAP;
        }
        bx += GROUP_GAP - BUTTON_GAP;
    }
    Some(Toolbar {
        rect: Rect { x, y, w, h },
        buttons,
        monitor,
    })
}

impl Toolbar {
    pub fn button_at(&self, p: (i32, i32)) -> Option<Button> {
        self.buttons.iter().find(|(_, r)| r.contains(p)).map(|(b, _)| *b)
    }

    /// The settings panel, on whichever side of the toolbar has room.
    pub fn panel(&self) -> SettingsPanel {
        let h = 2 * PAD + PANEL_TITLE + 2 * PANEL_ROW;
        let x = (self.rect.right() - PANEL_WIDTH)
            .min(self.monitor.right() - PANEL_WIDTH - SCREEN_MARGIN)
            .max(self.monitor.x + SCREEN_MARGIN);
        let above = self.rect.y - PAD - h;
        let y = if above >= self.monitor.y + SCREEN_MARGIN {
            above
        } else {
            self.rect.bottom() + PAD
        };
        let row = |index: i32| Rect {
            x: x + PAD,
            y: y + PAD + PANEL_TITLE + index * PANEL_ROW,
            w: PANEL_WIDTH - 2 * PAD,
            h: PANEL_ROW,
        };
        SettingsPanel {
            rect: Rect { x, y, w: PANEL_WIDTH, h },
            options: [
                (AfterSelect::Edit, "Open the editor", row(0)),
                (AfterSelect::Instant, "Copy right away, Shift to edit", row(1)),
            ],
        }
    }
}

impl SettingsPanel {
    pub const TITLE: &'static str = "After selecting";

    pub fn option_at(&self, p: (i32, i32)) -> Option<AfterSelect> {
        self.options.iter().find(|(_, _, r)| r.contains(p)).map(|(o, _, _)| *o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn monitors() -> Vec<Rect> {
        vec![
            rect(-1440, 0, 1440, 2560),
            rect(0, 0, 3440, 1440),
            rect(3440, 0, 1920, 1080),
        ]
    }

    fn inside(outer: &Rect, inner: &Rect) -> bool {
        inner.x >= outer.x && inner.y >= outer.y && inner.right() <= outer.right() && inner.bottom() <= outer.bottom()
    }

    #[test]
    fn sits_below_the_selection_when_there_is_room() {
        let sel = rect(1000, 200, 400, 300);
        let bar = place(&sel, &monitors()).unwrap();
        assert_eq!(bar.rect.y, 514);
        assert_eq!(bar.rect.x + bar.rect.w / 2, 1200);
    }

    #[test]
    fn goes_above_when_the_selection_reaches_the_bottom() {
        let sel = rect(1000, 1000, 400, 440);
        let bar = place(&sel, &monitors()).unwrap();
        assert!(bar.rect.bottom() < sel.y);
    }

    #[test]
    fn goes_inside_for_a_full_monitor() {
        let sel = rect(0, 0, 3440, 1440);
        let bar = place(&sel, &monitors()).unwrap();
        assert!(inside(&sel, &bar.rect));
    }

    #[test]
    fn stays_on_the_monitor_near_a_side_edge() {
        let sel = rect(3400, 100, 39, 100);
        let bar = place(&sel, &monitors()).unwrap();
        assert!(inside(&monitors()[1], &bar.rect));
    }

    #[test]
    fn a_spanning_selection_uses_the_monitor_with_most_of_it() {
        let sel = rect(-100, 200, 900, 300);
        let bar = place(&sel, &monitors()).unwrap();
        assert!(inside(&monitors()[1], &bar.rect));
    }

    #[test]
    fn every_button_is_inside_the_toolbar_and_findable() {
        let bar = place(&rect(1000, 200, 400, 300), &monitors()).unwrap();
        assert_eq!(bar.buttons.len(), 9);
        for (button, r) in &bar.buttons {
            assert!(inside(&bar.rect, r));
            assert_eq!(bar.button_at((r.x + 1, r.y + 1)), Some(*button));
        }
    }

    #[test]
    fn panel_stays_on_screen_and_finds_its_options() {
        let bar = place(&rect(0, 0, 3440, 60), &monitors()).unwrap();
        let panel = bar.panel();
        assert!(inside(&monitors()[1], &panel.rect));
        for (option, _, r) in &panel.options {
            assert!(inside(&panel.rect, r));
            assert_eq!(panel.option_at((r.x + 1, r.y + 1)), Some(*option));
        }
    }
}
