import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function Input({value,defaultValue,placeholder,iconLeft,iconRight,type='text',mono,size='md',invalid,disabled,onChange,style,...rest}){
  const [focus,setFocus]=React.useState(false);
  const h=size==='lg'?'var(--control-height-lg)':'var(--control-height)';
  const bd=invalid?'var(--status-danger)':focus?'var(--border-accent)':'var(--border-default)';
  return (
    <div style={{display:'flex',alignItems:'center',gap:8,height:h,padding:'0 10px',
      background:'var(--surface-sunken)',border:`var(--border-width) solid ${bd}`,
      borderRadius:'var(--radius-md)',opacity:disabled?0.55:1,
      boxShadow:focus?'var(--shadow-focus)':'none',transition:'var(--transition-control)',...style}}>
      {iconLeft?<Icon name={iconLeft} size={15} color="var(--text-muted)"/>:null}
      <input type={type} value={value} defaultValue={defaultValue} placeholder={placeholder}
        disabled={disabled} onChange={onChange} onFocus={()=>setFocus(true)} onBlur={()=>setFocus(false)}
        style={{flex:1,minWidth:0,border:'none',outline:'none',background:'transparent',
          color:'var(--text-body)',font:mono?'var(--type-meta)':'var(--type-small)'}} {...rest}/>
      {iconRight?<Icon name={iconRight} size={15} color="var(--text-muted)"/>:null}
    </div>
  );
}
