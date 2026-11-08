# Compatibility

cot is developed and used on Arch Linux with Hyprland.

| Environment                                                                  | Status                                               |
| ---------------------------------------------------------------------------- | ---------------------------------------------------- |
| Hyprland                                                                     | Supported, including window snapping                 |
| Other Wayland compositors with wlr-layer-shell and screen capture for `grim` | May work, not tested. Window snapping is unavailable |
| GNOME and other compositors without those protocols                          | Not supported                                        |
| X11                                                                          | Not supported                                        |

## Monitors

Multiple monitors with different resolutions, positions and rotations are supported, and a selection can span them. cot has mostly been used at a display scale of 1. Other scales are handled in the code, so please report anything that looks off.

## Other distributions

Nothing in cot is specific to Arch apart from the package names. On other distributions you need the same [dependencies](../tutorial/installation.md#dependencies), with their development headers for GTK 4 and gtk4-layer-shell, and a recent Rust toolchain.
