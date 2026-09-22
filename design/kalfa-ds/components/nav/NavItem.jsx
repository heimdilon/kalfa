import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function NavItem({icon,children,active,badge,depth=0,onClick,style,...rest}){
  const [h,setH]=React.useState(false);
  return (
    <button type="button" onClick={onClick} onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'flex',alignItems:'center',gap:'var(--space-4)',width:'100%',height:'var(--row-height)',
        padding:`0 var(--space-6) 0 ${10+depth*14}px`,border:0,borderRadius:'var(--radius-md)',cursor:'pointer',
        font:`${active?'var(--fw-semibold)':'var(--fw-regular)'} var(--text-small)/1 var(--font-sans)`,
        color:active?'var(--text-title)':h?'var(--text-body)':'var(--text-secondary)',
        background:active?'var(--surface-selected)':h?'var(--surface-raised)':'transparent',
        boxShadow:active?'var(--shadow-accent-edge)':'none',transition:'var(--transition-control)',textAlign:'left',...style}} {...rest}>
      {icon?<Icon name={icon} size={18} color={active?'var(--text-accent)':'currentColor'}/>:null}
      <span style={{flex:1,overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{children}</span>
      {badge!=null?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{badge}</span>:null}
    </button>
  );
}