import React from 'react';
import {Icon} from './Icon.jsx';

export function IconButton({icon,label,size='md',variant='ghost',active,disabled,onClick,style,...rest}){
  const box=size==='sm'?26:size==='lg'?40:32;
  const gl=size==='sm'?14:size==='lg'?18:16;
  const [h,setH]=React.useState(false);
  const isPrimary=variant==='primary';
  return (
    <button type="button" aria-label={label} title={label} disabled={disabled} onClick={onClick}
      onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'inline-flex',alignItems:'center',justifyContent:'center',width:box,height:box,
        borderRadius:'var(--radius-md)',cursor:disabled?'not-allowed':'pointer',
        background:isPrimary?(h&&!disabled?'var(--action-primary-hover)':'var(--action-primary)'):(active?'var(--surface-selected)':h&&!disabled?'var(--surface-raised)':'transparent'),
        color:isPrimary?'var(--action-primary-fg)':active?'var(--text-accent)':'var(--text-secondary)',
        border:isPrimary?'1px solid transparent':`var(--border-width) solid ${active?'var(--border-accent)':'transparent'}`,
        opacity:disabled?0.45:1,transition:'var(--transition-control)',...style}} {...rest}>
      <Icon name={icon} size={gl}/>
    </button>
  );
}
