# FAQ

## Why not use `grim` and `slurp` directly?

`slurp` opens its selection layer before `grim` takes the picture, so the pointer leaves whatever you were hovering. cot captures first. See [How it works](advanced/how-it-works.md).

## Does cot send my screenshots anywhere?

No. Screenshots go to your clipboard or to a file, and cot has no network code.

## Is the mouse cursor included?

Not by default. Use [`--cursor`](reference/command-line.md) to include it.

## Are black bars safe for sensitive information?

Yes. A black bar replaces the pixels with solid black in the final image. Blur and pixelate are weaker. See [Redacting](guide/redacting.md).

## Can I go back to instant screenshots?

Yes. Open the [settings panel](guide/annotating.md#settings) and choose **Copy right away, Shift to edit**, or set [`after_select`](reference/configuration.md#after_select) to `"instant"`.

## Can I change the colors?

Yes. Choose a [theme](reference/themes.md) and override any of its colors in the [config file](reference/configuration.md).

## Can I pick a different color for each annotation?

Not yet. All annotations use the theme's annotation color.

## Does cot work outside Hyprland?

Possibly, on other Wayland compositors with layer-shell support, but without window snapping. See [Compatibility](reference/compatibility.md).
