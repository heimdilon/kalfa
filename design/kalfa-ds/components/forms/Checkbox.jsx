import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function Checkbox({checked,indeterminate,label,hint,disabled,onChange,style}){
  const on=checked||indeterminate;
  return (
    <label style={{display:'inline-flex',alignItems:'center',gap:9,cursor:disabled?'not-allowed':'pointer',
      opacity:disabled?0.5:1,...style}} onClick={()=>!disabled&&onChange&&onChange(!checked)}>
      <span style={{display:'inline-flex',alignItems:'center',justifyContent:'center',width:17,height:17,flex:'none',
        borderRadius:'var(--radius-xs)',
        background:on?'var(--action-primary)':'var(--surface-sunken)',
        border:`var(--border-width) solid ${on?'var(--action-primary)':'var(--border-strong)'}`,
        color:'var(--action-primary-fg)',transition:'var(--transition-control)'}}>
        {indeterminate?<Icon name="minus" size={12} strokeWidth={3}/>:checked?<Icon name="check" size={12} strokeWidth={3}/>:null}
      </span>
      {label?<span style={{font:'var(--type-small)',color:'var(--text-body)'}}>{label}</span>:null}
      {hint?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{hint}</span>:null}
    </label>
  );
}
