import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function PanelHeader({icon,title,subtitle,actions,style,...rest}){
  return (
    <header style={{display:'flex',alignItems:'center',gap:'var(--space-5)',height:'var(--titlebar-height)',
      flex:'0 0 auto',padding:'0 var(--space-8)',background:'var(--bg-panel)',
      borderBottom:'var(--border-width) solid var(--border-subtle)',...style}} {...rest}>
      {icon?<Icon name={icon} size={20} color="var(--text-muted)"/>:null}
      <div style={{minWidth:0,display:'flex',alignItems:'baseline',gap:'var(--space-5)'}}>
        <span style={{font:'var(--type-title)',color:'var(--text-title)',letterSpacing:'var(--tracking-tight)',
          overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{title}</span>
        {subtitle?<span style={{font:'var(--type-meta)',color:'var(--text-muted)',whiteSpace:'nowrap'}}>{subtitle}</span>:null}
      </div>
      <div style={{marginLeft:'auto',display:'flex',alignItems:'center',gap:'var(--space-3)'}}>{actions}</div>
    </header>
  );
}