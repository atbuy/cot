# Redacting

The toolbar has three tools for hiding parts of a screenshot. Pick a tool, then drag over the area.

| Tool          | Result                    |
| ------------- | ------------------------- |
| **Black bar** | A solid black rectangle   |
| **Blur**      | A heavy blur              |
| **Pixelate**  | Large single-color blocks |

!!! warning "Use the black bar for sensitive information"

    A black bar replaces every pixel underneath with black, so nothing remains to recover. Blurred or pixelated text can sometimes be reconstructed. Use blur and pixelate to reduce distraction, and the black bar for passwords, tokens and personal details.

## What ends up in the image

Redactions are written directly into the final image. The PNG has no layers and no hidden copy of the original pixels.

Blur and pixelate are computed only from the pixels inside their own box, so nothing beside the box can show through.

While you drag a blur or pixelate you see an outline, and the effect appears when you release the mouse. A black bar is shown as you drag.

## How redactions stack

- Blur and pixelate sit directly on the screenshot, so arrows and text stay visible on top of them, even if drawn first.
- Black bars stack with annotations in drawing order. A later bar covers an earlier annotation, and anything drawn after the bar sits on top of it.

!!! tip "Check before sharing"

    Only the area inside a redaction is hidden. Make sure the bar covers everything, then undo with ++ctrl+z++ and redraw if it doesn't.
