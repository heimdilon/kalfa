One-line: the container for anything that happened — tool calls, summaries, panels.

```jsx
<Card icon="file-text" title="Dosya okundu" subtitle="src/middleware/auth.ts" actions={<Badge tone="ok" dot>Tamam</Badge>}>
  …
</Card>
```

12px radius, hairline border, no drop shadow in the dark app — separation comes from surface value. `tone="accent"` (copper border) marks a card awaiting the user.
