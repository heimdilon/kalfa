import React from 'react';
import {IconTile} from '../core/IconTile.jsx';
import {Button} from '../core/Button.jsx';

export function TurnSummary({title='Özet',items=[],time,onUndo,undoLabel='Geri al',note,style}){
  return (
    <section style={{display:'flex',gap:12,padding:'14px 16px',...style}}>
      <IconTile icon="file-check" size={32}/>
      <div style={{flex:1,minWidth:0}}>
        <div style={{display:'flex',alignItems:'center',gap:12,marginBottom:8}}>
          <span style={{font:'var(--fw-semibold) var(--text-small)/1 var(--font-sans)',color:'var(--text-title)'}}>{title}</span>
          <span style={{flex:1}}/>
          {onUndo?<Button size="sm" iconLeft="rotate-ccw" onClick={onUndo}>{undoLabel}</Button>:null}
          {time?<span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{time}</span>:null}
        </div>
        <ul style={{margin:0,paddingLeft:18,display:'flex',flexDirection:'column',gap:6}}>
          {items.map((it,i)=><li key={i} style={{font:'var(--type-body)',color:'var(--text-body)',textWrap:'pretty'}}>{it}</li>)}
        </ul>
        {note?<p style={{margin:'10px 0 0',font:'var(--type-meta)',color:'var(--text-muted)'}}>{note}</p>:null}
      </div>
    </section>
  );
}
