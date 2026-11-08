# How it works

## Why the hover state survives

A hover state ends when another surface appears under the pointer. A pipeline such as `grim -g "$(slurp)"` runs `slurp` first, so its selection layer takes the pointer before `grim` captures anything.

cot reverses that order:

1. It runs `grim` before creating any window, capturing every monitor in one frame.
2. It reads the visible windows from `hyprctl`.
3. It opens one fullscreen overlay per monitor, showing the frozen frame.
4. Everything after that happens on the frozen frame.
5. It crops the frame, flattens your annotations into it, and encodes a PNG.

## Multiple monitors

`grim` renders the whole layout into a single image, at the highest scale of any monitor. cot works in the compositor's logical coordinates and converts to image pixels only when cropping, so monitors with different resolutions, positions and rotations need no special handling.

If your monitors do not form a rectangle, the empty parts of the layout are black. A selection or `--full` capture that covers those parts includes the black area.

## One instance at a time

cot holds an abstract socket while it runs. Pressing the keybind again while the overlay is open does nothing, so a second run can't capture the first run's overlay.

## What you see is what you get

The overlay and the final image use the same drawing code for annotations and the same function for blur and pixelate. The magnifier is the exception, as it always shows the unedited frame.
