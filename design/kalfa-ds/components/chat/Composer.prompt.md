One-line: the session composer — prompt field with the model and permission-mode selectors welded into the same frame.

```jsx
<Composer model="glm-5.3" mode="Sıkı" onSend={send} />
```

Never separate the mode selector from the input: seeing the active permission mode at the moment of sending is a product promise, not a layout choice.
