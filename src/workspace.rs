//! Routes pointer and keyboard input between the toolbar, the annotation tools
//! and the selection. Free of GTK so it can be tested without a display.

use crate::editor::{Editor, Shape, Tool};
use crate::geometry::Rect;
use crate::selection::Selector;
use crate::settings::{AfterSelect, Settings};
use crate::toolbar::{self, Button, Hover, Toolbar};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    SaveSettings,
    Cancel,
    Finish(Rect),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Escape,
    Enter,
    Backspace,
    Char(char),
    Undo,
    Redo,
}

pub struct Workspace {
    selector: Selector,
    editor: Editor,
    settings: Settings,
    monitors: Vec<Rect>,
    settings_open: bool,
    hover: Option<Hover>,
    tooltip: Option<Button>,
    /// Set by a click, so a button's tooltip stays away until it is re-entered.
    tooltip_dismissed: bool,
}

impl Workspace {
    pub fn new(monitors: Vec<Rect>, bounds: Rect, settings: Settings) -> Self {
        Workspace {
            selector: Selector::new(monitors.clone(), bounds),
            editor: Editor::new(),
            settings,
            monitors,
            settings_open: false,
            hover: None,
            tooltip: None,
            tooltip_dismissed: false,
        }
    }

    pub fn selection(&self) -> Option<Rect> {
        self.selector.selection()
    }

    pub fn editor(&self) -> &Editor {
        &self.editor
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    pub fn hover(&self) -> Option<Hover> {
        self.hover
    }

    pub fn tooltip(&self) -> Option<Button> {
        self.tooltip
    }

    /// The button whose tooltip should appear once the hover delay has passed.
    pub fn pending_tooltip(&self) -> Option<Button> {
        match self.hover {
            Some(Hover::Button(button)) if !self.tooltip_dismissed && self.tooltip != Some(button) => Some(button),
            _ => None,
        }
    }

    /// Shows the tooltip, if the pointer is still waiting on that button.
    pub fn show_tooltip(&mut self, button: Button) {
        if self.pending_tooltip() == Some(button) {
            self.tooltip = Some(button);
        }
    }

    fn is_dragging(&self) -> bool {
        self.selector.is_dragging() || self.editor.is_drawing()
    }

    pub fn shows_handles(&self) -> bool {
        self.selector.shows_handles() && self.editor.tool() == Tool::Select
    }

    pub fn toolbar(&self) -> Option<Toolbar> {
        if !self.selector.shows_handles() {
            return None;
        }
        toolbar::place(&self.selector.selection()?, &self.monitors)
    }

    fn hover_at(&self, p: (i32, i32)) -> Option<Hover> {
        if self.is_dragging() {
            return None;
        }
        let bar = self.toolbar()?;
        if self.settings_open {
            if let Some(option) = bar.panel().option_at(p) {
                return Some(Hover::Option(option));
            }
        }
        bar.button_at(p).map(Hover::Button)
    }

    fn over_chrome(&self, p: (i32, i32)) -> bool {
        !self.is_dragging()
            && self
                .toolbar()
                .is_some_and(|bar| bar.rect.contains(p) || (self.settings_open && bar.panel().rect.contains(p)))
    }

    /// Returns whether anything visible changed.
    pub fn pointer_moved(&mut self, p: (i32, i32)) -> bool {
        self.selector.set_pointer(p);
        let pointer = self.selector.pointer();
        if self.editor.is_drawing() {
            self.editor.drag(pointer);
            return true;
        }
        let hover = self.hover_at(pointer);
        let hover_changed = hover != self.hover;
        self.hover = hover;
        if hover_changed {
            self.tooltip = None;
            self.tooltip_dismissed = false;
        }
        hover_changed || self.selector.is_dragging()
    }

    fn activate(&mut self, button: Button) {
        match button {
            Button::Tool(tool) => self.editor.set_tool(tool),
            Button::Undo => self.editor.undo(),
            Button::Redo => self.editor.redo(),
            Button::Settings => self.settings_open = !self.settings_open,
        }
    }

    pub fn press(&mut self) -> Outcome {
        self.tooltip = None;
        self.tooltip_dismissed = true;
        let p = self.selector.pointer();
        if let Some(bar) = self.toolbar() {
            if self.settings_open {
                let panel = bar.panel();
                if panel.rect.contains(p) {
                    return match panel.option_at(p) {
                        Some(option) if option != self.settings.after_select => {
                            self.settings.after_select = option;
                            Outcome::SaveSettings
                        }
                        _ => Outcome::Continue,
                    };
                }
                if bar.button_at(p) != Some(Button::Settings) {
                    self.settings_open = false;
                    if !bar.rect.contains(p) {
                        return Outcome::Continue;
                    }
                }
            }
            if let Some(button) = bar.button_at(p) {
                self.activate(button);
                return Outcome::Continue;
            }
            if bar.rect.contains(p) {
                return Outcome::Continue;
            }
        }

        if self.editor.tool() == Tool::Select {
            self.selector.press();
        } else if self.selector.selection().is_some_and(|sel| sel.contains(p)) {
            self.editor.press(p);
        } else {
            self.editor.commit();
        }
        Outcome::Continue
    }

    pub fn release(&mut self, shift_held: bool) -> Outcome {
        if self.editor.is_drawing() {
            self.editor.commit();
            return Outcome::Continue;
        }
        let is_new = self.selector.release();
        if is_new && self.settings.after_select == AfterSelect::Instant && !shift_held {
            return Outcome::Finish(self.selector.confirmed());
        }
        Outcome::Continue
    }

    pub fn key(&mut self, key: Key) -> Outcome {
        match key {
            Key::Escape if self.editor.is_typing() => self.editor.discard_in_progress(),
            Key::Escape if self.settings_open => self.settings_open = false,
            Key::Escape => return Outcome::Cancel,
            Key::Enter if self.editor.is_typing() => self.editor.commit(),
            Key::Enter => {
                self.editor.commit();
                return Outcome::Finish(self.selector.confirmed());
            }
            Key::Backspace => self.editor.backspace(),
            Key::Char(c) => self.editor.type_char(c),
            Key::Undo => self.editor.undo(),
            Key::Redo => self.editor.redo(),
        }
        Outcome::Continue
    }

    /// The annotations to flatten into the final image.
    pub fn finished_shapes(&self) -> Vec<Shape> {
        self.editor.done().to_vec()
    }

    /// The CSS cursor name for what a press would do here.
    pub fn cursor(&self) -> &'static str {
        let p = self.selector.pointer();
        if self.hover_at(p).is_some() {
            return "pointer";
        }
        if self.over_chrome(p) {
            return "default";
        }
        match self.editor.tool() {
            Tool::Select => self.selector.cursor(),
            tool => match self.selector.selection() {
                Some(sel) if sel.contains(p) && tool == Tool::Text => "text",
                Some(sel) if sel.contains(p) => "crosshair",
                _ => "default",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SELECTION: Rect = Rect { x: 100, y: 200, w: 300, h: 300 };

    fn empty(settings: Settings) -> Workspace {
        let monitors = vec![
            Rect { x: -1440, y: 0, w: 1440, h: 2560 },
            Rect { x: 0, y: 0, w: 3440, h: 1440 },
            Rect { x: 3440, y: 0, w: 1920, h: 1080 },
        ];
        Workspace::new(monitors, Rect { x: -1440, y: 0, w: 6800, h: 2560 }, settings)
    }

    fn drag(ws: &mut Workspace, from: (i32, i32), to: (i32, i32), shift: bool) -> Outcome {
        ws.pointer_moved(from);
        ws.press();
        ws.pointer_moved(to);
        ws.release(shift)
    }

    fn selected() -> Workspace {
        let mut ws = empty(Settings::default());
        drag(&mut ws, (100, 200), (400, 500), false);
        ws
    }

    fn click_button(ws: &mut Workspace, button: Button) -> Outcome {
        let bar = ws.toolbar().unwrap();
        let (_, r) = *bar.buttons.iter().find(|(b, _)| *b == button).unwrap();
        ws.pointer_moved((r.x + 2, r.y + 2));
        let outcome = ws.press();
        ws.release(false);
        outcome
    }

    #[test]
    fn toolbar_appears_once_a_selection_settles() {
        let mut ws = empty(Settings::default());
        ws.pointer_moved((100, 200));
        ws.press();
        ws.pointer_moved((400, 500));
        assert!(ws.toolbar().is_none());
        ws.release(false);
        assert!(ws.toolbar().is_some());
    }

    #[test]
    fn edit_mode_does_not_finish_on_release() {
        let mut ws = empty(Settings::default());
        assert_eq!(drag(&mut ws, (100, 200), (400, 500), false), Outcome::Continue);
    }

    #[test]
    fn instant_mode_finishes_on_release() {
        let mut ws = empty(Settings { after_select: AfterSelect::Instant });
        assert_eq!(drag(&mut ws, (100, 200), (400, 500), false), Outcome::Finish(SELECTION));
    }

    #[test]
    fn instant_mode_with_shift_opens_the_editor() {
        let mut ws = empty(Settings { after_select: AfterSelect::Instant });
        assert_eq!(drag(&mut ws, (100, 200), (400, 500), true), Outcome::Continue);
        assert!(ws.toolbar().is_some());
    }

    #[test]
    fn instant_mode_does_not_finish_when_adjusting() {
        let mut ws = empty(Settings { after_select: AfterSelect::Instant });
        drag(&mut ws, (100, 200), (400, 500), true);
        assert_eq!(drag(&mut ws, (250, 350), (260, 360), false), Outcome::Continue);
    }

    #[test]
    fn a_tool_draws_inside_the_selection_without_moving_it() {
        let mut ws = selected();
        click_button(&mut ws, Button::Tool(Tool::Rectangle));
        drag(&mut ws, (150, 250), (300, 400), false);
        assert_eq!(ws.finished_shapes(), [Shape::Rectangle { a: (150, 250), b: (300, 400) }]);
        assert_eq!(ws.selection(), Some(SELECTION));
    }

    #[test]
    fn a_tool_ignores_presses_outside_the_selection() {
        let mut ws = selected();
        click_button(&mut ws, Button::Tool(Tool::Arrow));
        drag(&mut ws, (1000, 1000), (1100, 1100), false);
        assert!(ws.finished_shapes().is_empty());
        assert_eq!(ws.selection(), Some(SELECTION));
    }

    #[test]
    fn handles_only_show_with_the_select_tool() {
        let mut ws = selected();
        assert!(ws.shows_handles());
        click_button(&mut ws, Button::Tool(Tool::Arrow));
        assert!(!ws.shows_handles());
    }

    #[test]
    fn toolbar_clicks_do_not_touch_the_selection() {
        let mut ws = selected();
        click_button(&mut ws, Button::Settings);
        click_button(&mut ws, Button::Tool(Tool::Arrow));
        assert_eq!(ws.selection(), Some(SELECTION));
        assert_eq!(ws.editor().tool(), Tool::Arrow);
    }

    #[test]
    fn undo_and_redo_buttons_work() {
        let mut ws = selected();
        click_button(&mut ws, Button::Tool(Tool::Freehand));
        drag(&mut ws, (150, 250), (300, 400), false);
        click_button(&mut ws, Button::Undo);
        assert!(ws.finished_shapes().is_empty());
        click_button(&mut ws, Button::Redo);
        assert_eq!(ws.finished_shapes().len(), 1);
    }

    fn typing() -> Workspace {
        let mut ws = selected();
        click_button(&mut ws, Button::Tool(Tool::Text));
        ws.pointer_moved((150, 250));
        ws.press();
        ws.release(false);
        ws.key(Key::Char('h'));
        ws.key(Key::Char('i'));
        ws
    }

    #[test]
    fn enter_commits_text_before_it_copies() {
        let mut ws = typing();
        assert_eq!(ws.key(Key::Enter), Outcome::Continue);
        assert_eq!(ws.finished_shapes(), [Shape::Text { at: (150, 250), text: "hi".into() }]);
        assert_eq!(ws.key(Key::Enter), Outcome::Finish(SELECTION));
    }

    #[test]
    fn escape_drops_the_text_but_keeps_the_screenshot_open() {
        let mut ws = typing();
        assert_eq!(ws.key(Key::Escape), Outcome::Continue);
        assert!(ws.finished_shapes().is_empty());
        assert_eq!(ws.key(Key::Escape), Outcome::Cancel);
    }

    #[test]
    fn typing_is_ignored_without_a_text_box() {
        let mut ws = selected();
        ws.key(Key::Char('x'));
        assert!(ws.finished_shapes().is_empty());
    }

    fn hover_button(ws: &mut Workspace, button: Button) {
        let bar = ws.toolbar().unwrap();
        let (_, r) = *bar.buttons.iter().find(|(b, _)| *b == button).unwrap();
        ws.pointer_moved((r.x + 2, r.y + 2));
    }

    #[test]
    fn hovering_waits_for_the_delay_before_showing_a_tooltip() {
        let mut ws = selected();
        hover_button(&mut ws, Button::Undo);
        assert_eq!(ws.pending_tooltip(), Some(Button::Undo));
        assert_eq!(ws.tooltip(), None);
        ws.show_tooltip(Button::Undo);
        assert_eq!(ws.tooltip(), Some(Button::Undo));
        assert_eq!(ws.pending_tooltip(), None);
    }

    #[test]
    fn leaving_hides_the_tooltip() {
        let mut ws = selected();
        hover_button(&mut ws, Button::Undo);
        ws.show_tooltip(Button::Undo);
        ws.pointer_moved((2000, 1000));
        assert_eq!(ws.tooltip(), None);
        assert_eq!(ws.pending_tooltip(), None);
    }

    #[test]
    fn a_late_timer_for_another_button_shows_nothing() {
        let mut ws = selected();
        hover_button(&mut ws, Button::Undo);
        hover_button(&mut ws, Button::Redo);
        ws.show_tooltip(Button::Undo);
        assert_eq!(ws.tooltip(), None);
        assert_eq!(ws.pending_tooltip(), Some(Button::Redo));
    }

    #[test]
    fn clicking_dismisses_the_tooltip_until_the_button_is_reentered() {
        let mut ws = selected();
        click_button(&mut ws, Button::Settings);
        assert_eq!(ws.tooltip(), None);
        assert_eq!(ws.pending_tooltip(), None);
        ws.pointer_moved((2000, 1000));
        hover_button(&mut ws, Button::Settings);
        assert_eq!(ws.pending_tooltip(), Some(Button::Settings));
    }

    #[test]
    fn dragging_over_the_toolbar_does_not_hover_it() {
        let mut ws = selected();
        let bar = ws.toolbar().unwrap();
        let (_, r) = bar.buttons[1];
        ws.pointer_moved((250, 350));
        ws.press();
        ws.pointer_moved((r.x + 2, r.y + 2));
        assert_eq!(ws.hover(), None);
        assert_eq!(ws.pending_tooltip(), None);
    }

    fn click_option(ws: &mut Workspace, wanted: AfterSelect) -> Outcome {
        let panel = ws.toolbar().unwrap().panel();
        let (_, _, r) = *panel.options.iter().find(|(o, _, _)| *o == wanted).unwrap();
        ws.pointer_moved((r.x + 2, r.y + 2));
        let outcome = ws.press();
        ws.release(false);
        outcome
    }

    #[test]
    fn choosing_a_setting_asks_to_save_it() {
        let mut ws = selected();
        click_button(&mut ws, Button::Settings);
        assert_eq!(click_option(&mut ws, AfterSelect::Instant), Outcome::SaveSettings);
        assert_eq!(ws.settings().after_select, AfterSelect::Instant);
        assert_eq!(click_option(&mut ws, AfterSelect::Instant), Outcome::Continue);
    }

    #[test]
    fn escape_closes_settings_before_cancelling() {
        let mut ws = selected();
        click_button(&mut ws, Button::Settings);
        assert_eq!(ws.key(Key::Escape), Outcome::Continue);
        assert!(!ws.settings_open());
        assert_eq!(ws.key(Key::Escape), Outcome::Cancel);
    }

    #[test]
    fn clicking_away_closes_settings_without_changing_the_selection() {
        let mut ws = selected();
        click_button(&mut ws, Button::Settings);
        drag(&mut ws, (2000, 1000), (2100, 1100), false);
        assert!(!ws.settings_open());
        assert_eq!(ws.selection(), Some(SELECTION));
    }
}
