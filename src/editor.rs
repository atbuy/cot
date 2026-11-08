//! Annotations and their undo history, in global logical coordinates so they
//! stay put on the frozen screen when the selection is moved.

type Point = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Arrow,
    Rectangle,
    Freehand,
    Highlighter,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Arrow { from: Point, to: Point },
    Rectangle { a: Point, b: Point },
    Freehand(Vec<Point>),
    Highlighter(Vec<Point>),
    Text { at: Point, text: String },
}

/// Shorter drags than this are slips of the mouse, not arrows or rectangles.
const MIN_EXTENT: i32 = 3;

impl Shape {
    fn is_worth_keeping(&self) -> bool {
        let spans = |a: &Point, b: &Point| (a.0 - b.0).abs() >= MIN_EXTENT || (a.1 - b.1).abs() >= MIN_EXTENT;
        match self {
            Shape::Arrow { from: a, to: b } | Shape::Rectangle { a, b } => spans(a, b),
            Shape::Freehand(points) | Shape::Highlighter(points) => points.len() >= 2,
            Shape::Text { text, .. } => !text.trim().is_empty(),
        }
    }
}

pub struct Editor {
    tool: Tool,
    done: Vec<Shape>,
    undone: Vec<Shape>,
    in_progress: Option<Shape>,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            tool: Tool::Select,
            done: Vec::new(),
            undone: Vec::new(),
            in_progress: None,
        }
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.commit();
        self.tool = tool;
    }

    pub fn is_drawing(&self) -> bool {
        self.in_progress.is_some() && !self.is_typing()
    }

    pub fn is_typing(&self) -> bool {
        matches!(self.in_progress, Some(Shape::Text { .. }))
    }

    pub fn press(&mut self, p: Point) {
        self.commit();
        self.in_progress = match self.tool {
            Tool::Select => None,
            Tool::Arrow => Some(Shape::Arrow { from: p, to: p }),
            Tool::Rectangle => Some(Shape::Rectangle { a: p, b: p }),
            Tool::Freehand => Some(Shape::Freehand(vec![p])),
            Tool::Highlighter => Some(Shape::Highlighter(vec![p])),
            Tool::Text => Some(Shape::Text { at: p, text: String::new() }),
        };
    }

    pub fn drag(&mut self, p: Point) {
        match &mut self.in_progress {
            Some(Shape::Arrow { to: end, .. }) | Some(Shape::Rectangle { b: end, .. }) => *end = p,
            Some(Shape::Freehand(points)) | Some(Shape::Highlighter(points)) => {
                if points.last() != Some(&p) {
                    points.push(p);
                }
            }
            _ => {}
        }
    }

    /// Finishes whatever is in progress, dropping it if it came to nothing.
    pub fn commit(&mut self) {
        if let Some(shape) = self.in_progress.take() {
            if shape.is_worth_keeping() {
                self.done.push(shape);
                self.undone.clear();
            }
        }
    }

    pub fn discard_in_progress(&mut self) {
        self.in_progress = None;
    }

    pub fn type_char(&mut self, c: char) {
        if let Some(Shape::Text { text, .. }) = &mut self.in_progress {
            text.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(Shape::Text { text, .. }) = &mut self.in_progress {
            text.pop();
        }
    }

    pub fn can_undo(&self) -> bool {
        self.in_progress.is_some() || !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Takes back the shape in progress if there is one, otherwise the last.
    pub fn undo(&mut self) {
        if self.in_progress.take().is_none() {
            if let Some(shape) = self.done.pop() {
                self.undone.push(shape);
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(shape) = self.undone.pop() {
            self.done.push(shape);
        }
    }

    pub fn done(&self) -> &[Shape] {
        &self.done
    }

    pub fn in_progress(&self) -> Option<&Shape> {
        self.in_progress.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_arrow() -> Editor {
        let mut e = Editor::new();
        e.set_tool(Tool::Arrow);
        e.press((0, 0));
        e.drag((50, 50));
        e.commit();
        e
    }

    #[test]
    fn draws_a_shape_following_the_pointer() {
        assert_eq!(with_arrow().done(), [Shape::Arrow { from: (0, 0), to: (50, 50) }]);
    }

    #[test]
    fn a_click_with_a_drag_tool_leaves_nothing() {
        let mut e = Editor::new();
        for tool in [Tool::Arrow, Tool::Rectangle, Tool::Freehand, Tool::Highlighter] {
            e.set_tool(tool);
            e.press((10, 10));
            e.commit();
        }
        assert!(e.done().is_empty());
    }

    #[test]
    fn undo_and_redo_walk_the_history() {
        let mut e = with_arrow();
        e.undo();
        assert!(e.done().is_empty());
        assert!(e.can_redo());
        e.redo();
        assert_eq!(e.done().len(), 1);
    }

    #[test]
    fn a_new_shape_clears_redo() {
        let mut e = with_arrow();
        e.undo();
        e.press((0, 0));
        e.drag((9, 9));
        e.commit();
        assert!(!e.can_redo());
    }

    #[test]
    fn text_is_typed_edited_and_committed() {
        let mut e = Editor::new();
        e.set_tool(Tool::Text);
        e.press((5, 6));
        assert!(e.is_typing());
        "héy".chars().for_each(|c| e.type_char(c));
        e.backspace();
        e.commit();
        assert_eq!(e.done(), [Shape::Text { at: (5, 6), text: "hé".into() }]);
    }

    #[test]
    fn blank_text_is_dropped() {
        let mut e = Editor::new();
        e.set_tool(Tool::Text);
        e.press((5, 6));
        e.type_char(' ');
        e.commit();
        assert!(e.done().is_empty());
    }

    #[test]
    fn switching_tools_commits_the_text_being_typed() {
        let mut e = Editor::new();
        e.set_tool(Tool::Text);
        e.press((5, 6));
        e.type_char('a');
        e.set_tool(Tool::Arrow);
        assert_eq!(e.done().len(), 1);
    }

    #[test]
    fn undo_while_typing_only_discards_the_text() {
        let mut e = with_arrow();
        e.set_tool(Tool::Text);
        e.press((5, 6));
        e.type_char('a');
        e.undo();
        assert!(!e.is_typing());
        assert_eq!(e.done().len(), 1);
    }

    #[test]
    fn freehand_skips_repeated_points() {
        let mut e = Editor::new();
        e.set_tool(Tool::Freehand);
        e.press((1, 1));
        e.drag((1, 1));
        e.drag((2, 2));
        e.commit();
        assert_eq!(e.done(), [Shape::Freehand(vec![(1, 1), (2, 2)])]);
    }
}
