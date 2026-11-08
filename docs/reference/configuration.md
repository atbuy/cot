# Configuration

cot reads its settings from:

```text
~/.config/cot/config.toml
```

If `XDG_CONFIG_HOME` is set to an absolute path, the file is `$XDG_CONFIG_HOME/cot/config.toml` instead.

The file is optional, and anything left out keeps its default. It is read each time cot starts, so a change applies from the next screenshot.

## Example

```toml
theme = "ultraviolet"
after_select = "edit"
magnifier = true
save_folder = "~/Pictures/Screenshots"

[colors]
selection = "#FF8800"
```

## Settings

### `theme`

The built-in color theme. See [Themes](themes.md) for the list. Default: `"hot-coral"`.

Also available in the [settings panel](../guide/annotating.md#settings), with a preview.

### `after_select`

What happens when you finish a new selection. Default: `"edit"`.

| Value       | Behavior                                                         |
| ----------- | ---------------------------------------------------------------- |
| `"edit"`    | Keeps the selection on screen with the toolbar                   |
| `"instant"` | Copies right away. Hold ++shift++ as you release to edit instead |

Also available in the [settings panel](../guide/annotating.md#settings).

### `magnifier`

Whether to show the [magnifier](../guide/selecting.md#the-magnifier) while drawing or resizing a selection. `true` or `false`. Default: `true`.

Also available in the settings panel.

### `save_folder`

Where ++ctrl+s++ and the **Save** button put screenshots. It must be an absolute path or start with `~/`. Default: `"~/Pictures/Screenshots"`.

### `colors`

A table that overrides individual colors of the theme. Each value is written as `"#RRGGBB"`.

| Key          | Used for                                 |
| ------------ | ---------------------------------------- |
| `selection`  | Selection border, handles, active button |
| `surface`    | Toolbar, labels and panel backgrounds    |
| `text`       | Toolbar icons and label text             |
| `annotation` | Everything you draw                      |

## Editing the file

The settings panel changes only its own settings in this file. Comments and everything else stay as you wrote them. The file is written in place, so it can be a symlink into a dotfiles repository.

## Mistakes in the file

A problem in the file never stops cot from opening.

- An invalid value or unknown setting keeps its default and prints a warning.
- An invalid color falls back to the theme's color.
- An unknown theme falls back to Hot Coral.
- A file that is not valid TOML means all defaults, and the settings panel refuses to overwrite it, so your text is not lost.

Warnings only appear in a terminal. Run `cot` from one if a change doesn't seem to apply.
