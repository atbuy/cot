# Themes

Choose a theme in the [config file](configuration.md):

```toml
theme = "graphite-mono"
```

## Built-in themes

<table>
  <thead>
    <tr><th>Name</th><th>Selection</th><th>Surface</th><th>Text</th><th>Annotation</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>hot-coral</code> (default)</td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FF4D6D"></span> <code>#FF4D6D</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#1C1216"></span> <code>#1C1216</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FFECEF"></span> <code>#FFECEF</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#4DD8FF"></span> <code>#4DD8FF</code></td>
    </tr>
    <tr>
      <td><code>signal-amber</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FFB000"></span> <code>#FFB000</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#17140F"></span> <code>#17140F</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#F5EDE0"></span> <code>#F5EDE0</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FF5C39"></span> <code>#FF5C39</code></td>
    </tr>
    <tr>
      <td><code>electric-mint</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#2EF2B3"></span> <code>#2EF2B3</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#0E1A1A"></span> <code>#0E1A1A</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#E6FFF7"></span> <code>#E6FFF7</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FF4F8B"></span> <code>#FF4F8B</code></td>
    </tr>
    <tr>
      <td><code>ultraviolet</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#B388FF"></span> <code>#B388FF</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#17122B"></span> <code>#17122B</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#F0EBFF"></span> <code>#F0EBFF</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FFD166"></span> <code>#FFD166</code></td>
    </tr>
    <tr>
      <td><code>graphite-mono</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FFFFFF"></span> <code>#FFFFFF</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#1A1A1A"></span> <code>#1A1A1A</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#F2F2F2"></span> <code>#F2F2F2</code></td>
      <td><span class="swatch" style="display:inline-block;width:1.2em;height:1.2em;vertical-align:middle;border:1px solid #8888;background:#FF3B30"></span> <code>#FF3B30</code></td>
    </tr>
  </tbody>
</table>

!!! note "Light backgrounds"

    A bright selection color is harder to see over white pages. Signal Amber, Electric Mint and especially Graphite Mono are affected. The dimmed area around the selection still shows where it ends.

## Your own colors

Pick a theme, then override any of its colors under `[colors]`. Each override replaces only that color:

```toml
theme = "ultraviolet"

[colors]
selection = "#FF8800"
```

This gives you Ultraviolet with an orange selection. The keys are listed in the [configuration reference](configuration.md#colors).

The order in the file doesn't matter. `[colors]` always applies on top of the theme.
