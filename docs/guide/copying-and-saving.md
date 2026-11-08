# Copying and saving

When you are done, either action includes your annotations and redactions and closes cot.

| Action                            | Result                                              |
| --------------------------------- | --------------------------------------------------- |
| ++enter++ or ++ctrl+c++           | Copies a PNG to the clipboard                       |
| ++ctrl+s++ or the **Save** button | Saves a PNG to a file. The clipboard is not touched |

With nothing selected, these take all monitors.

## Where files go

Saved screenshots go to `~/Pictures/Screenshots`, which is created if it is missing. To use another folder, set [`save_folder`](../reference/configuration.md#save_folder).

Files are named after the time they were taken:

```text
cot-2026-11-08_18-30-45.png
```

If that name is taken, a number is added, as in `cot-2026-11-08_18-30-45-1.png`. Saving never overwrites an existing file.

!!! note "No confirmation"

    cot closes without showing a message. If a save fails, for example on a full disk, the error only goes to the terminal. See [Troubleshooting](../reference/troubleshooting.md).

## Writing to a specific path

The `-o` option sends the result to a path you choose, instead of the clipboard or the screenshots folder:

```bash
cot -o ~/Desktop/shot.png
```

With `-o`, both copying and saving write to that path, and an existing file there is replaced. See the [command line reference](../reference/command-line.md).
