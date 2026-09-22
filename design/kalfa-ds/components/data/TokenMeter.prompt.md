One-line: segmented usage bar with a mono legend — token breakdown per request, context fill.

```jsx
<TokenMeter total={4096} used={2341} segments={[
  {label:'Sistem',value:312,color:'var(--status-info)'},
  {label:'Kullanıcı',value:1842,color:'var(--copper-400)'},
  {label:'Araçlar',value:187,color:'var(--status-warn)'}]} />
```

Tokens are the *certain* number in Kalfa; credits are only an observation. Show tokens plainly, and never imply a settled cost.
