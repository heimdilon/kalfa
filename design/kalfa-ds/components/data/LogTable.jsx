import React from 'react';
import {Icon} from '../core/Icon.jsx';
import {IconTile} from '../core/IconTile.jsx';

export function LogTable({columns=[],rows=[],activeId,onSelect,style}){
  return (
    <div style={{border:'var(--border-width) solid var(--border-subtle)',borderRadius:'var(--radius-lg)',
      overflow:'hidden',background:'var(--surface-card)',...style}}>
      <div style={{display:'grid',gridTemplateColumns:columns.map(c=>c.width||'1fr').join(' ')+' 28px',
        gap:12,padding:'11px 16px',background:'var(--surface-sunken)',
        borderBottom:'var(--border-width) solid var(--border-subtle)'}}>
        {columns.map(c=><span key={c.key} style={{font:'var(--fw-semibold) var(--text-micro)/1 var(--font-sans)',color:'var(--text-muted)'}}>{c.label}</span>)}
        <span/>
      </div>
      {rows.map(r=>(
        <LogRow key={r.id} row={r} columns={columns} active={r.id===activeId} onSelect={onSelect}/>
      ))}
    </div>
  );
}

function LogRow({row,columns,active,onSelect}){
  const [h,setH]=React.useState(false);
  return (
    <div onClick={()=>onSelect&&onSelect(row)} onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'grid',gridTemplateColumns:columns.map(c=>c.width||'1fr').join(' ')+' 28px',gap:12,
        alignItems:'center',padding:'12px 16px',cursor:'pointer',
        background:active?'var(--surface-selected)':h?'var(--surface-raised)':'transparent',
        boxShadow:active?'var(--shadow-accent-edge)':'none',
        borderBottom:'var(--border-width) solid var(--border-subtle)',
        transition:'var(--transition-control)'}}>
      {columns.map(c=>{
        const v=row[c.key];
        if(c.key==='purpose')return (
          <span key={c.key} style={{display:'flex',alignItems:'center',gap:10,minWidth:0}}>
            <IconTile icon={row.purposeIcon||'message-square'} tone={row.purposeTone||'neutral'} size={28}/>
            <span style={{minWidth:0}}>
              <span style={{display:'block',font:'var(--fw-medium) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-body)'}}>{v}</span>
              {row.purposeNote?<span style={{display:'block',font:'var(--type-meta)',color:'var(--text-muted)',marginTop:2}}>{row.purposeNote}</span>:null}
            </span>
          </span>);
        return <span key={c.key} style={{minWidth:0,font:c.mono?'var(--type-meta)':'var(--type-small)',
          color:c.muted?'var(--text-muted)':'var(--text-secondary)',
          overflow:'hidden',textOverflow:'ellipsis'}}>{v}</span>;
      })}
      <Icon name="chevron-right" size={15} color="var(--text-muted)"/>
    </div>
  );
}
