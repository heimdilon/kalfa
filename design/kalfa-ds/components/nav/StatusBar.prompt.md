One-line: the always-visible bottom bar of facts — isolation, model, tokens, connection.

```jsx
<StatusBar right={<StatusItem icon="activity">2.341 token</StatusItem>}>
  <StatusItem icon="shield-check" tone="ok">Yalıtılmış</StatusItem>
  <StatusItem icon="box">glm-5.3</StatusItem>
</StatusBar>
```

Everything here is mono and muted; only a tone colour breaks out, and only for a real state change. The bar never holds actions.