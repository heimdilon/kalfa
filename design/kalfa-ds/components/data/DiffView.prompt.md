One-line: two-gutter unified diff — the evidence shown on every write approval.

```jsx
<DiffView hunkHeader="@@ -42,10 +42,18 @@ async fn main() -> Result<()> {"
  lines={[{old:44,neu:44,text:''},{kind:'del',old:45,text:'app.fs = Fs::new();'},{kind:'add',neu:48,text:'app.fs = Fs::with_permission_manager(…);'}]} />
```

Added rows use a green wash, removed a red wash, with a slightly darker gutter of the same hue. No inline word-level highlight — Kalfa keeps diffs quiet and readable.
