# Installation

cot is built from source. It is developed on Arch Linux with Hyprland. See [Compatibility](../reference/compatibility.md) for what else may work.

## Dependencies

| Package              | Why cot needs it                                  |
| -------------------- | ------------------------------------------------- |
| `grim`               | Captures the screen                               |
| `wl-clipboard`       | Provides `wl-copy`, used to copy to the clipboard |
| `gtk4`               | Draws the overlay and toolbar                     |
| `gtk4-layer-shell`   | Places the overlay above everything else          |
| `rust`, `base-devel` | Build the program                                 |
| `git`                | Downloads the source                              |

```bash
sudo pacman -S --needed base-devel git rust grim wl-clipboard gtk4 gtk4-layer-shell
```

Window snapping also uses `hyprctl`, which comes with Hyprland.

## Build and install

```bash
git clone https://github.com/atbuy/cot
cd cot
make install
```

`make install` builds a release binary and copies it to `~/.local/bin/cot`.

!!! tip "Update"

    To update, run `git pull` and `make install` again. The keybind keeps launching the old build until you do.

## Install somewhere else

Set `PREFIX` to change the location. The binary goes in `$PREFIX/bin`:

```bash
make install PREFIX=/opt/cot
```

## Set up a keybind

Add one line to your Hyprland config. Use the full path, because a compositor's `PATH` may not include `~/.local/bin`.

=== "hyprland.conf"

    ```ini
    bind = CONTROL, F12, exec, ~/.local/bin/cot
    ```

=== "hyprland.lua"

    ```lua
    hl.bind("CONTROL + F12", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.local/bin/cot"))
    ```

Any key combination works. You can add a second bind with [`--full`](../reference/command-line.md) to take all monitors without selecting.

## Uninstall

From the source folder:

```bash
make uninstall
```

If you installed with a custom `PREFIX`, pass the same one again. Your settings stay in `~/.config/cot/config.toml` until you delete them.
