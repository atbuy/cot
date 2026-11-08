# Getting started

This page is the short path from nothing to a working screenshot key. Each step links to more detail.

## 1. Install

On Arch Linux, install the dependencies and build cot:

```bash
sudo pacman -S --needed base-devel git rust grim wl-clipboard gtk4 gtk4-layer-shell
git clone https://github.com/atbuy/cot
cd cot
make install
```

This puts the program at `~/.local/bin/cot`. See [Installation](installation.md) for other options.

## 2. Bind it to a key

Add a keybind to your Hyprland config:

=== "hyprland.conf"

    ```ini
    bind = CONTROL, F12, exec, ~/.local/bin/cot
    ```

=== "hyprland.lua"

    ```lua
    hl.bind("CONTROL + F12", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.local/bin/cot"))
    ```

## 3. Take a screenshot

1. Press ++ctrl+f12++. The screen freezes and dims.
2. Drag across the area you want.
3. Press ++enter++ to copy it to the clipboard.

Paste it anywhere that accepts images. Press ++esc++ at any point to cancel.

## Next steps

- Walk through the editor in [Your first screenshot](first-screenshot.md).
- See every way to pick an area in [Selecting](../guide/selecting.md).
- Keep [Keyboard and mouse](../reference/shortcuts.md) handy.
