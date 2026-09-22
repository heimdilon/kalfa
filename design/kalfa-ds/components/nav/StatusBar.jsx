import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function StatusBar({left,right,children,style,...rest}){
  return (
    <footer style={{display:'flex',alignItems:'center',gap:'var(--space-8)',height:'var(--statusbar-height)',
      flex:'0 0 auto',padding:'0 var(--space-8)',background:'var(--bg-rail)',
      borderTop:'var(--border-width) solid var(--border-subtle)',font:'var(--type-meta)',color:'var(--text-muted)',...style}} {...rest}>
      {left}{children}
      <div style={{marginLeft:'auto',display:'flex',alignItems:'center',gap:'var(--space-8)'}}>{right}</div>
    </footer>
  );
}

export function StatusItem({icon,tone,children,style}){
  const c=tone?{ok:'var(--status-ok)',warn:'var(--status-warn)',danger:'var(--status-danger)',info:'var(--status-info)',accent:'var(--text-accent)'}[tone]:'var(--text-muted)';
  return (
    <span style={{display:'inline-flex',alignItems:'center',gap:'var(--space-3)',color:c,whiteSpace:'nowrap',...style}}>
      {icon?<Icon name={icon} size={14}/>:null}{children}
    </span>
  );
}