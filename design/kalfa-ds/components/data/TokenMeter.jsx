import React from 'react';

export function TokenMeter({segments=[],total,used,style}){
  const sum=segments.reduce((a,s)=>a+s.value,0)||1;
  const fill=total?Math.min(1,(used??sum)/total):1;
  return (
    <div style={{...style}}>
      <div style={{display:'flex',height:6,borderRadius:'var(--radius-pill)',overflow:'hidden',
        background:'var(--surface-sunken)',border:'var(--border-width) solid var(--border-subtle)'}}>
        <div style={{display:'flex',width:`${fill*100}%`,minWidth:2}}>
          {segments.map((s,i)=>(
            <span key={i} style={{width:`${(s.value/sum)*100}%`,background:s.color||'var(--copper-400)'}}/>
          ))}
        </div>
      </div>
      <div style={{display:'flex',flexWrap:'wrap',gap:'6px 16px',marginTop:10}}>
        {segments.map((s,i)=>(
          <span key={i} style={{display:'inline-flex',alignItems:'center',gap:7,font:'var(--type-meta)',color:'var(--text-secondary)'}}>
            <span style={{width:7,height:7,borderRadius:'50%',background:s.color||'var(--copper-400)'}}/>
            {s.label} ({s.value.toLocaleString('tr-TR')})
          </span>))}
      </div>
    </div>
  );
}
