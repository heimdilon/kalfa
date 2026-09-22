One-line: record of one tool execution inside the conversation.

```jsx
<ToolCallCard tool="Dosya okundu" target="src/middleware/auth.ts" meta="412 satır · 18 KB" status="done">
  <CodeBlock code={src} truncated />
</ToolCallCard>
```

Titles are past-tense results, not imperatives. `status="unknown"` is a real state Kalfa shows after a crash — never quietly re-run.
