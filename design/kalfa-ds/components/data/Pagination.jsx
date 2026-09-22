import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function Pagination({page=1,pages=1,onChange,style}){
  const list=[];for(let i=1;i<=Math.min(pages,5);i++)list.push(i);
  const Btn=({children,active,dis,onClick,label})=>(
    <button type="button" aria-label={label} disabled={dis} onClick={onClick}
      style={{minWidth:28,height:28,padding:'0 6px',borderRadius:'var(--radius-sm)',
        background:active?'var(--surface-selected)':'transparent',
        border:`var(--border-width) solid ${active?'var(--border-accent)':'transparent'}`,
        color:active?'var(--text-accent)':dis?'var(--text-muted)':'var(--text-secondary)',
        font:'var(--type-small)',cursor:dis?'default':'pointer',opacity:dis?0.5:1}}>{children}</button>);
  return (
    <nav style={{display:'flex',alignItems:'center',gap:4,...style}}>
      <Btn label="Önceki" dis={page<=1} onClick={()=>onChange&&onChange(page-1)}><Icon name="chevron-left" size={14}/></Btn>
      {list.map(n=><Btn key={n} active={n===page} onClick={()=>onChange&&onChange(n)}>{n}</Btn>)}
      {pages>5?<span style={{color:'var(--text-muted)',padding:'0 4px'}}>···</span>:null}
      <Btn label="Sonraki" dis={page>=pages} onClick={()=>onChange&&onChange(page+1)}><Icon name="chevron-right" size={14}/></Btn>
    </nav>
  );
}
