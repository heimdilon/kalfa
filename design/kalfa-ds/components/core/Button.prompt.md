One-line: the action button — use `approve`/`caution`/`danger` only on permission decisions, never for ordinary navigation.

```jsx
<Button variant="approve" iconLeft="check">Tümünü onayla</Button>
<Button variant="caution" iconLeft="check">Seçilenleri onayla</Button>
<Button variant="danger" iconLeft="x">Reddet</Button>
<Button variant="primary" size="lg">Kabul ediyorum</Button>
```

Only one `primary` per view. Labels are Turkish, sentence case, verb-first ("Geri al", "Kabul etmiyorum"). Hover lightens the fill; press darkens it and nudges 0.5px down — no scale, no shadow.
