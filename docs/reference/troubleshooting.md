# Troubleshooting

Most problems print a message when cot runs from a terminal, which a keybind hides. Start there:

```bash
~/.local/bin/cot
```

## Nothing happens when I press the keybind

- Check that the path in the keybind is right. Use the full path, as in [Installation](../tutorial/installation.md#set-up-a-keybind).
- Check whether cot is already open. A second run exits without doing anything.
- Run it from a terminal and read the error.

## `could not run grim` or `could not run wl-copy`

The tool is not installed or not on your `PATH`:

```bash
sudo pacman -S --needed grim wl-clipboard
```

## A new feature or shortcut doesn't work

The keybind launches the installed copy, which only changes when you install. From the source folder:

```bash
make install
```

## Windows don't light up when I hover

- Window snapping needs Hyprland. Run cot from a terminal: if `hyprctl` can't be reached, it prints `window snapping is off`.
- Highlighting only happens before anything is selected.

## The wrong window lights up

Hyprland does not report exact stacking order, so overlapping floating windows can be guessed wrong. Drag a region instead.

## A setting doesn't apply

Run cot from a terminal and look for lines starting with `cot: config:`. They name what is wrong. See [Mistakes in the file](configuration.md#mistakes-in-the-file).

## My setting isn't saved from the toolbar

If the config file isn't valid TOML, cot won't overwrite it. Run cot from a terminal to see `could not save settings`, then fix the file.

## A saved screenshot is missing

Saving prints an error to the terminal if it fails, for example when `save_folder` can't be written. Check that the folder exists or can be created, and that the disk has space.

## The screenshot has black areas

If your monitors don't form a rectangle, the empty parts of the layout are black. They appear in any selection or `--full` capture that covers them.

## Accents don't work in the text tool

Dead keys and input methods are not supported yet.
