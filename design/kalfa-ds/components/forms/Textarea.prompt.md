One-line: multi-line field; carries its own control tray in `footer` (that is how the composer is built).

```jsx
<Textarea accent rows={2} placeholder="Bir şey sorun veya bir görev tanımlayın…"
  footer={<><Select value="glm-5.3" icon="box"/><Select value="Sıkı" icon="shield"/></>} />
<Textarea rows={2} placeholder="Reddetme nedeninizi buraya yazabilirsiniz…" />
```

The composer's copper border is permanent (`accent`), not a focus state — it is the one always-live input in the app.
