import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function ThoughtPanel({steps,summary,defaultOpen=false,children,style}){
  const [open,setOpen]=React.useState(defaultOpen);
  return (
    <section style={{background:'var(--surface-card)',border:'var(--border-width) solid var(--border-subtle)',
      borderRadius:'var(--radius-xl)',overflow:'hidden',...style}}>
      <button type="button" onClick={()=>setOpen(o=>!o)}
        style={{display:'flex',alignItems:'center',gap:10,width:'100%',padding:'12px 14px',
          background:'transparent',border:'none',cursor:'pointer',textAlign:'left'}}>
        <Icon name={open?'chevron-down':'chevron-right'} size={15} color="var(--text-muted)"/>
        <span style={{flex:1}}>
          <span style={{display:'block',font:'var(--fw-semibold) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-title)'}}>Düşünce</span>
          <span style={{display:'block',font:'var(--type-meta)',color:'var(--text-muted)',marginTop:3}}>
            {summary||`Bu yanıt için izlenen adımlar (${steps??0} adım)`}</span>
        </span>
        <Icon name="chevron-right" size={15} color="var(--text-muted)"/>
      </button>
      {open?<div style={{padding:'0 14px 14px 39px',font:'var(--type-small)',color:'var(--text-secondary)',
        borderTop:'var(--border-width) solid var(--border-subtle)',paddingTop:12}}>{children}</div>:null}
    </section>
  );
}
