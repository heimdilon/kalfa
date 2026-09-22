import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function Select({value,options=[],icon,tone='default',size='md',disabled,onChange,style}){
  const [open,setOpen]=React.useState(false);
  const [h,setH]=React.useState(false);
  const height=size==='sm'?28:size==='lg'?'var(--control-height-lg)':'var(--control-height)';
  const fg=tone==='accent'?'var(--text-accent)':'var(--text-body)';
  return (
    <div style={{position:'relative',display:'inline-block',...style}}>
      <button type="button" disabled={disabled} onClick={()=>setOpen(o=>!o)}
        onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
        style={{display:'inline-flex',alignItems:'center',gap:8,height,padding:'0 10px',
          background:h&&!disabled?'var(--surface-raised)':'var(--surface-sunken)',
          border:`var(--border-width) solid ${open?'var(--border-accent)':'var(--border-default)'}`,
          borderRadius:'var(--radius-md)',color:fg,font:'var(--type-small)',
          cursor:disabled?'not-allowed':'pointer',opacity:disabled?0.55:1,
          transition:'var(--transition-control)'}}>
        {icon?<Icon name={icon} size={15} color={tone==='accent'?'var(--text-accent)':'var(--text-muted)'}/>:null}
        <span style={{flex:1,textAlign:'left'}}>{value}</span>
        <Icon name="chevron-down" size={14} color="var(--text-muted)"/>
      </button>
      {open?(
        <div style={{position:'absolute',top:'calc(100% + 6px)',left:0,minWidth:'100%',zIndex:40,
          background:'var(--surface-card)',border:'var(--border-width) solid var(--border-default)',
          borderRadius:'var(--radius-lg)',boxShadow:'var(--shadow-overlay)',padding:4,whiteSpace:'nowrap'}}>
          {options.map(o=>{
            const label=typeof o==='string'?o:o.label;const hint=typeof o==='string'?null:o.hint;
            const sel=label===value;
            return (
              <button key={label} type="button"
                onClick={()=>{setOpen(false);onChange&&onChange(label);}}
                style={{display:'flex',alignItems:'center',gap:10,width:'100%',padding:'7px 9px',
                  background:sel?'var(--surface-selected)':'transparent',border:'none',
                  borderRadius:'var(--radius-sm)',color:sel?'var(--text-accent)':'var(--text-body)',
                  font:'var(--type-small)',cursor:'pointer',textAlign:'left'}}>
                <span style={{flex:1}}>{label}</span>
                {hint?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{hint}</span>:null}
                {sel?<Icon name="check" size={14}/>:null}
              </button>);
          })}
        </div>
      ):null}
    </div>
  );
}
