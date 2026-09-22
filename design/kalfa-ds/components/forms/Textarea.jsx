import React from 'react';

export function Textarea({value,defaultValue,placeholder,rows=3,accent,disabled,onChange,footer,style,...rest}){
  const [focus,setFocus]=React.useState(false);
  const on=accent||focus;
  return (
    <div style={{background:'var(--surface-sunken)',
      border:`${on?'var(--border-width-accent)':'var(--border-width)'} solid ${on?'var(--border-accent)':'var(--border-default)'}`,
      borderRadius:'var(--radius-lg)',padding:'10px 12px',opacity:disabled?0.55:1,
      transition:'var(--transition-control)',...style}}>
      <textarea value={value} defaultValue={defaultValue} placeholder={placeholder} rows={rows}
        disabled={disabled} onChange={onChange} onFocus={()=>setFocus(true)} onBlur={()=>setFocus(false)}
        style={{width:'100%',border:'none',outline:'none',background:'transparent',resize:'none',
          color:'var(--text-body)',font:'var(--type-body)'}} {...rest}/>
      {footer?<div style={{display:'flex',alignItems:'center',gap:8,marginTop:8}}>{footer}</div>:null}
    </div>
  );
}
