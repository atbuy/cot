[![CI](https://github.com/atbuy/cot/actions/workflows/ci.yml/badge.svg)](https://github.com/atbuy/cot/actions/workflows/ci.yml)
[![Docs](https://github.com/atbuy/cot/actions/workflows/docs.yml/badge.svg)](https://github.com/atbuy/cot/actions/workflows/docs.yml)

<img src="docs/assets/logo.svg" alt="cot logo" width="96" />

# cot

`cot` is a screenshot tool for Wayland. It freezes the screen the moment you press the keybind, then lets you select, annotate and redact on the frozen image, so hover states and open menus stay in the screenshot.

Documentation: [https://atbuy.github.io/cot/](https://atbuy.github.io/cot/)

## Quick start

On Arch Linux with Hyprland:

```bash
sudo pacman -S --needed base-devel git rust grim wl-clipboard gtk4 gtk4-layer-shell
git clone https://github.com/atbuy/cot
cd cot
make install
```

Bind `~/.local/bin/cot` to a key in your Hyprland config, press it, drag across an area, and press Enter to copy it.
