import React from 'react';

export function MetaList({items=[],labelWidth=112,dense,style}){
  return (
    <dl style={{display:'grid',gridTemplateColumns:`${labelWidth}px 1fr`,rowGap:dense?6:10,columnGap:12,margin:0,...style}}>
      {items.map((it,i)=>(
        <React.Fragment key={i}>
          <dt style={{font:'var(--type-small)',color:'var(--text-muted)'}}>{it.label}</dt>
          <dd style={{margin:0,font:it.mono?'var(--type-meta)':'var(--type-small)',
            color:it.tone==='accent'?'var(--text-accent)':'var(--text-body)',
            overflowWrap:'anywhere'}}>{it.value}</dd>
        </React.Fragment>))}
    </dl>
  );
}
