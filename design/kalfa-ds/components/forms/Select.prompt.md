One-line: compact dropdown — model picker, permission mode, log filters, approval scope.

```jsx
<Select icon="box" value="glm-5.3" options={[{label:'glm-5.3',hint:'derin'},{label:'deepseek-v4-flash',hint:'dengeli'},{label:'qwen3.8-flash-next',hint:'hızlı'},'auto']} />
<Select icon="shield" value="Sıkı" options={['İnceleme','Sıkı','Proje','Serbest']} />
```

The trigger shows the current value as a chip, not a label + value pair. Menu is a floating card with an overlay shadow — the only place Kalfa uses a heavy shadow in dark mode.
