One-line: flat, indent-driven workspace tree for the right-hand project panel.

```jsx
<FileTree activePath="src/middleware/auth.ts" items={[
  {name:'src',type:'dir',open:true,depth:0},
  {name:'middleware',type:'dir',open:true,depth:1},
  {name:'auth.ts',path:'src/middleware/auth.ts',depth:2}]} />
```

26px rows, copper folder glyphs, the active file on the strong copper tint. Pass a pre-flattened array with explicit `depth` — no nesting.
