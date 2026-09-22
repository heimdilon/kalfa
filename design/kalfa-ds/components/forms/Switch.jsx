import React from 'react';

export function Switch({checked,label,hint,disabled,onChange,style}){
  return (
    <label style={{display:'inline-flex',alignItems:'center',gap:10,cursor:disabled?'not-allowed':'pointer',
      opacity:disabled?0.5:1,...style}} onClick={()=>!disabled&&onChange&&onChange(!checked)}>
      {label?<span style={{display:'flex',flexDirection:'column',gap:2}}>
        <span style={{font:'var(--type-small)',color:'var(--text-body)'}}>{label}</span>
        {hint?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{hint}</span>:null}
      </span>:null}
      <span style={{position:'relative',width:40,height:22,flex:'none',borderRadius:'var(--radius-pill)',
        background:checked?'var(--action-primary)':'var(--warm-600)',
        border:'var(--border-width) solid '+(checked?'var(--action-primary)':'var(--border-strong)'),
        transition:'var(--transition-control)'}}>
        <span style={{position:'absolute',top:2,left:checked?20:2,width:16,height:16,borderRadius:'50%',
          background:checked?'var(--action-primary-fg)':'var(--text-secondary)',
          transition:'left var(--dur-fast) var(--ease-standard)'}}/>
      </span>
    </label>
  );
}
