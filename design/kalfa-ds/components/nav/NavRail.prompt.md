One-line: the app's left rail container, plus `NavSection` for its all-caps group labels.

```jsx
<NavRail header={<Wordmark/>} footer={<NavItem icon="settings">Ayarlar</NavItem>}>
  <NavItem icon="messages-square" active>Oturumlar</NavItem>
  <NavSection label="Çalışma alanı">
    <NavItem icon="folder-tree">Dosyalar</NavItem>
  </NavSection>
</NavRail>
```

Fixed width (213px), rail background one step darker than the app, hairline right border. Never a drop shadow.