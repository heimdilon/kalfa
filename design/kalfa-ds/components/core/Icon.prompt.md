One-line: renders a Lucide glyph; the only icon primitive in Kalfa — never hand-draw SVG paths.

```jsx
<Icon name="shield-check" size={16} />
<Icon name="folder" size={18} color="var(--text-muted)" />
```

Requires the Lucide UMD script on the page: `<script src="https://unpkg.com/lucide@0.460.0/dist/umd/lucide.js"></script>`.
Sizes: 14 inline with 13px text · 16 buttons and list rows · 18 rail nav · 20 panel headers.
Color always comes from a text token (`currentColor` by default), never a hardcoded hex.
