import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function SessionRow({title,preview,time,status,active,onClick,style,...rest}){
  const [h,setH]=React.useState(false);
  const dot={running:'var(--status-warn)',done:'var(--status-ok)',failed:'var(--status-danger)',idle:'var(--text-muted)'}[status||'idle'];
  return (
    <button type="button" onClick={onClick} onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'flex',flexDirection:'column',gap:'var(--space-2)',width:'100%',textAlign:'left',
        padding:'var(--space-5) var(--space-6)',cursor:'pointer',border:0,borderRadius:'var(--radius-md)',
        background:active?'var(--surface-selected)':h?'var(--surface-raised)':'transparent',
        boxShadow:active?'var(--shadow-accent-edge)':'none',transition:'var(--transition-control)',...style}} {...rest}>
      <span style={{display:'flex',alignItems:'center',gap:'var(--space-4)',width:'100%'}}>
        {status?<span style={{width:6,height:6,borderRadius:'var(--radius-pill)',background:dot,flex:'0 0 auto'}}/>:null}
        <span style={{flex:1,minWidth:0,font:'var(--fw-medium) var(--text-small)/1.3 var(--font-sans)',
          color:active?'var(--text-title)':'var(--text-body)',overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{title}</span>
        {time?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{time}</span>:null}
      </span>
      {preview?<span style={{font:'var(--fw-regular) var(--text-micro)/1.45 var(--font-sans)',color:'var(--text-muted)',
        display:'-webkit-box',WebkitLineClamp:2,WebkitBoxOrient:'vertical',overflow:'hidden'}}>{preview}</span>:null}
    </button>
  );
}