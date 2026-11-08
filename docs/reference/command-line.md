# Command line

```text
cot [OPTIONS]
```

With no options, cot freezes the screen and opens the overlay.

| Option                  | Description                                                                |
| ----------------------- | -------------------------------------------------------------------------- |
| `--full`                | Skips selection and takes all monitors                                     |
| `-c`, `--cursor`        | Includes the mouse cursor in the screenshot                                |
| `-o`, `--output <PATH>` | Writes the PNG to this path instead of the clipboard or screenshots folder |
| `-h`, `--help`          | Prints help                                                                |
| `-V`, `--version`       | Prints the version                                                         |

## Examples

Take all monitors and copy to the clipboard:

```bash
cot --full
```

Take all monitors and write to a file:

```bash
cot --full -o ~/Desktop/everything.png
```

Select an area, with the cursor included:

```bash
cot --cursor
```

## Behavior

- `-o` replaces an existing file at that path. The ++ctrl+s++ save never overwrites one.
- `--full` never opens the overlay, so it copies to the clipboard unless `-o` is given.
- Cancelling exits with status `0` and writes nothing.
- Starting cot while it is already open exits immediately and does nothing.
- Errors are printed to the terminal, and cot exits with status `1`.
