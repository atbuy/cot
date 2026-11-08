# Installation

cot is developed on Arch Linux with Hyprland, and has prebuilt releases for Arch and for Ubuntu 26.04 with X11. See [Compatibility](../reference/compatibility.md) for what else may work.

## Install with the script

```bash title="Install latest"
curl -fsSL https://raw.githubusercontent.com/atbuy/cot/main/install.sh | sh
```

```bash title="Install a specific version"
curl -fsSL https://raw.githubusercontent.com/atbuy/cot/main/install.sh | sh -s -- --version v0.1.0
```

```bash title="Install system-wide"
curl -fsSL https://raw.githubusercontent.com/atbuy/cot/main/install.sh | sh -s -- --system
```

The script downloads the release for your distribution, verifies its checksum, and installs to `~/.local/bin/cot`, or `/usr/local/bin/cot` with `--system`. If a dependency is missing, it prints the command to install it.

To update later, run:

```bash
cot self update
```

## Build from source

### Dependencies

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

On Ubuntu 26.04 with X11, install these instead. `xclip` does the job of `wl-copy`, and the screen is captured without `grim`:

```bash
sudo apt install build-essential git pkg-config rustup libgtk-4-dev libgtk4-layer-shell-dev xclip
rustup default stable
```

### Build and install

```bash
git clone https://github.com/atbuy/cot
cd cot
make install
```

`make install` builds a release binary and copies it to `~/.local/bin/cot`.

!!! tip "Update"

    To update, run `git pull` and `make install` again. The keybind keeps launching the old build until you do.

### Install somewhere else

Set `PREFIX` to change the location. The binary goes in `$PREFIX/bin`:

```bash
make install PREFIX=/opt/cot
```

## Set up a keybind

Add one line to your window manager's config. Use the full path, because its `PATH` may not include `~/.local/bin`.

=== "hyprland.conf"

    ```ini
    bind = CONTROL, F12, exec, ~/.local/bin/cot
    ```

=== "i3 config"

    ```ini
    bindsym Control+F12 exec --no-startup-id ~/.local/bin/cot
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
