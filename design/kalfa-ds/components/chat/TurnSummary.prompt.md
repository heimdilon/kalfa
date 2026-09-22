One-line: closing block of a turn — what changed, plus the undo affordance when the change is reversible.

```jsx
<TurnSummary time="14:30" onUndo={undo} note="Kabuk etkileri geri alınamaz."
  items={['JWT doğrulama mantığı doğru, ancak token süresi hatalı.','Refresh token\'lar hash\'lenmeden saklanıyor.']} />
```

Only render `onUndo` when the turn is genuinely reversible; shell and network effects are not, and the `note` must say so.
