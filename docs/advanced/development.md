# Development

## Build and test

```bash
cargo build
cargo test
cargo clippy --all-targets
```

The tests don't need a display. Selection, toolbar and input handling are kept free of GTK so they can be exercised directly, and drawing is tested on in-memory images.

## Source layout

| File               | Purpose                                           |
| ------------------ | ------------------------------------------------- |
| `src/main.rs`      | Arguments, capture order and output               |
| `src/image.rs`     | The frame: capture, crop, blur, pixelate, PNG     |
| `src/overlay.rs`   | GTK windows and event wiring                      |
| `src/workspace.rs` | Routes input between toolbar, tools and selection |
| `src/selection.rs` | Drawing, moving and resizing a selection          |
| `src/editor.rs`    | Annotations and undo history                      |
| `src/toolbar.rs`   | Toolbar and settings panel layout                 |
| `src/magnifier.rs` | Magnifier placement                               |
| `src/render.rs`    | Cairo drawing and flattening                      |
| `src/settings.rs`  | The config file                                   |
| `src/palette.rs`   | Themes and active colors                          |
| `src/windows.rs`   | Reading windows from Hyprland                     |
| `src/geometry.rs`  | Rectangles and coordinate conversion              |
| `src/update.rs`    | `cot self update`                                 |
| `src/version.rs`   | Version information                               |

## Documentation

The site is built with [Zensical](https://zensical.org/) from the `docs` folder.

```bash
pip install zensical
make docs-serve
```

`make docs-build` writes the site to `site/`.

## Continuous integration

Two GitHub Actions workflows run on pushes to `main` and on pull requests:

- `ci.yml` runs clippy and the tests in an Arch Linux container.
- `docs.yml` builds the site, and publishes it to GitHub Pages on pushes to `main`.

Publishing needs GitHub Pages enabled for the repository, with **GitHub Actions** as its source.

## Releasing

Push a version tag to publish a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

`release.yml` runs clippy and the tests, builds for Arch and Ubuntu 26.04, and publishes the archives with a `checksums.txt`. The tag becomes the version shown by `cot version`. GTK is linked dynamically, so each distribution release needs its own build.
