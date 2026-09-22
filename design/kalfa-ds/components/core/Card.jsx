import React from 'react';
import {Icon} from './Icon.jsx';

export function Card({title,subtitle,icon,actions,tone='default',padded=true,selected,children,style}){
  const toneBorder=tone==='accent'?'var(--border-accent)':tone==='danger'?'color-mix(in oklab,var(--red-400) 40%,transparent)':selected?'var(--border-accent)':'var(--border-subtle)';
  return (
    <section style={{background:tone==='sunken'?'var(--surface-sunken)':'var(--surface-card)',
      border:`var(--border-width) solid ${toneBorder}`,borderRadius:'var(--radius-xl)',
      overflow:'hidden',...style}}>
      {(title||actions)?(
        <header style={{display:'flex',alignItems:'center',gap:10,padding:'12px 14px',
          borderBottom:children?'var(--border-width) solid var(--border-subtle)':'none'}}>
          {icon?<span style={{display:'flex',alignItems:'center',justifyContent:'center',width:28,height:28,
            borderRadius:'var(--radius-md)',background:'var(--surface-sunken)',color:'var(--text-secondary)',flex:'none'}}>
            <Icon name={icon} size={15}/></span>:null}
          <div style={{minWidth:0,flex:1}}>
            <div style={{font:'var(--fw-semibold) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-title)'}}>{title}</div>
            {subtitle?<div style={{font:'var(--type-meta)',color:'var(--text-muted)',marginTop:3}}>{subtitle}</div>:null}
          </div>
          {actions?<div style={{display:'flex',alignItems:'center',gap:6,flex:'none'}}>{actions}</div>:null}
        </header>
      ):null}
      {children?<div style={{padding:padded?'var(--pad-card)':0}}>{children}</div>:null}
    </section>
  );
}
