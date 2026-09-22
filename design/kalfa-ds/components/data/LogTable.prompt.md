One-line: the transmission-log table — one row per EVREN request, selectable into a detail panel.

```jsx
<LogTable activeId="req_8f3a" onSelect={setRow}
  columns={[{key:'time',label:'Zaman',width:'110px',mono:true},{key:'purpose',label:'Amaç',width:'1.4fr'},
            {key:'model',label:'Model',width:'130px',mono:true},{key:'tokens',label:'Token',width:'80px',mono:true}]}
  rows={[{id:'req_8f3a',time:'14:02:11',purpose:'Ana model',purposeNote:'Kod inceleme ve öneri',model:'glm-5.3',tokens:'2.341'}]} />
```

The selected row gets the copper tint plus a 3px copper left edge. Ids and numbers are always mono; Turkish thousands use a dot (2.341).
