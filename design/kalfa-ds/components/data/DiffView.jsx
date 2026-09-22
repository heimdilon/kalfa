import React from 'react';

export function DiffView({hunkHeader,lines=[],style}){
  return (
    <div style={{background:'var(--surface-sunken)',border:'var(--border-width) solid var(--border-subtle)',
      borderRadius:'var(--radius-md)',overflow:'hidden',font:'var(--type-code)',...style}}>
      {hunkHeader?<div style={{padding:'6px 14px',color:'var(--text-muted)',
        background:'color-mix(in oklab,var(--warm-200) 5%,transparent)',
        borderBottom:'var(--border-width) solid var(--border-subtle)'}}>{hunkHeader}</div>:null}
      <table style={{borderCollapse:'collapse',width:'100%'}}>
        <tbody>
          {lines.map((l,i)=>{
            const k=l.kind||'ctx';
            const bg=k==='add'?'var(--diff-add-bg)':k==='del'?'var(--diff-del-bg)':'transparent';
            const fg=k==='add'?'var(--diff-add-fg)':k==='del'?'var(--diff-del-fg)':'var(--code-punct)';
            const gut=k==='add'?'var(--diff-add-gutter)':k==='del'?'var(--diff-del-gutter)':'transparent';
            return (
              <tr key={i} style={{background:bg}}>
                <td style={{width:1,whiteSpace:'nowrap',textAlign:'right',padding:'0 8px 0 12px',color:'var(--warm-400)',userSelect:'none',background:gut}}>{l.old??''}</td>
                <td style={{width:1,whiteSpace:'nowrap',textAlign:'right',padding:'0 10px 0 4px',color:'var(--warm-400)',userSelect:'none',background:gut}}>{l.neu??''}</td>
                <td style={{width:1,padding:'0 8px',color:fg,userSelect:'none'}}>{k==='add'?'+':k==='del'?'−':' '}</td>
                <td style={{padding:'0 14px 0 0',whiteSpace:'pre',color:fg}}>{l.text}</td>
              </tr>);
          })}
        </tbody>
      </table>
    </div>
  );
}
