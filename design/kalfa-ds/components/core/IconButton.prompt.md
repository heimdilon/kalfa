One-line: square glyph-only control for toolbars, composer trays and panel headers.

```jsx
<IconButton icon="paperclip" label="Dosya ekle" />
<IconButton icon="send" label="Gönder" variant="primary" />
<IconButton icon="x" label="Kapat" size="sm" />
```

`label` is never optional — it is the only text a screen reader gets. `variant="primary"` is reserved for the composer's send button.
