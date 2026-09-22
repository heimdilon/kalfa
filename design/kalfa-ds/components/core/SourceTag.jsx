import React from 'react';
import {Icon} from './Icon.jsx';

const KALFA_SOURCE_KINDS={
  dosya:{icon:'file-text',label:'dosya'},
  kabuk:{icon:'terminal',label:'kabuk'},
  web:{icon:'globe',label:'web'},
  eklenti:{icon:'puzzle',label:'eklenti'},
  rag:{icon:'library',label:'rag'},
  mesaj:{icon:'message-square',label:null},
  gecmis:{icon:null,label:null}
};

/** The provenance chip: every piece of content sent to EVREN carries one. */
export function SourceTag({kind='dosya',value,live,muted,onClick,style}){
  const k=KALFA_SOURCE_KINDS[kind]||KALFA_SOURCE_KINDS.dosya;
  const [h,setH]=React.useState(false);
  const interactive=!!onClick;
  return (
    <span role={interactive?'button':undefined} tabIndex={interactive?0:undefined} onClick={onClick}
      onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'inline-flex',alignItems:'center',gap:6,height:24,padding:'0 9px',
        borderRadius:'var(--radius-sm)',font:'var(--fw-regular) var(--text-micro)/1 var(--font-mono)',
        letterSpacing:'var(--tracking-mono)',
        color:muted?'var(--text-muted)':'var(--text-secondary)',
        background:h&&interactive?'var(--surface-raised)':'var(--surface-sunken)',
        border:'var(--border-width) solid var(--border-subtle)',
        cursor:interactive?'pointer':'default',transition:'var(--transition-control)',
        whiteSpace:'nowrap',maxWidth:'100%',overflow:'hidden',textOverflow:'ellipsis',...style}}>
      {live?<span style={{width:6,height:6,borderRadius:'50%',background:'var(--status-ok)',flex:'none'}}/>
        :k.icon?<Icon name={k.icon} size={12} color="var(--text-muted)"/>:null}
      {k.label?<span style={{color:'var(--text-muted)'}}>{k.label}:</span>:null}
      <span>{value}</span>
    </span>
  );
}
