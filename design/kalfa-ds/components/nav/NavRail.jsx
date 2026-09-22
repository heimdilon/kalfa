import React from 'react';

export function NavRail({header,footer,children,width='var(--rail-width)',style,...rest}){
  return (
    <nav style={{display:'flex',flexDirection:'column',width,flex:'0 0 auto',height:'100%',
      background:'var(--bg-rail)',borderRight:'var(--border-width) solid var(--border-subtle)',...style}} {...rest}>
      {header?<div style={{padding:'var(--space-6) var(--space-6) var(--space-4)'}}>{header}</div>:null}
      <div style={{flex:1,overflowY:'auto',padding:'var(--space-2) var(--space-4)',display:'flex',flexDirection:'column',gap:'var(--space-1)'}}>{children}</div>
      {footer?<div style={{padding:'var(--space-4) var(--space-6)',borderTop:'var(--border-width) solid var(--border-subtle)'}}>{footer}</div>:null}
    </nav>
  );
}

export function NavSection({label,children,style}){
  return (
    <div style={{display:'flex',flexDirection:'column',gap:'var(--space-1)',marginTop:'var(--space-6)',...style}}>
      <div style={{font:'var(--type-label)',letterSpacing:'var(--tracking-label)',textTransform:'uppercase',
        color:'var(--text-muted)',padding:'0 var(--space-5) var(--space-3)'}}>{label}</div>
      {children}
    </div>
  );
}