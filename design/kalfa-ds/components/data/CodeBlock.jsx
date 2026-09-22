import React from 'react';

const KALFA_CODE_RULES=[
  [/(\/\/[^\n]*)/g,'var(--code-com)'],
  [/("(?:[^"\\]|\\.)*")/g,'var(--code-str)'],
  [/\b(import|from|export|const|let|mut|fn|async|await|return|if|else|try|catch|use|pub|impl|struct|enum|match|function|new|self)\b/g,'var(--code-kw)'],
  [/\b([A-Z][A-Za-z0-9_]*)\b/g,'var(--code-type)'],
  [/\b(\d+)\b/g,'var(--code-num)']
];
function paint(line){
  let parts=[{t:line,c:null}];
  for(const [re,color] of KALFA_CODE_RULES){
    const next=[];
    for(const p of parts){
      if(p.c){next.push(p);continue;}
      let last=0;p.t.replace(re,(m,g,off)=>{if(off>last)next.push({t:p.t.slice(last,off),c:null});next.push({t:m,c:color});last=off+m.length;return m;});
      if(last<p.t.length)next.push({t:p.t.slice(last),c:null});
    }
    parts=next;
  }
  return parts.map((p,i)=><span key={i} style={{color:p.c||'var(--code-punct)'}}>{p.t}</span>);
}

export function CodeBlock({code='',startLine=1,showLineNumbers=true,truncated,maxHeight,style}){
  const lines=String(code).replace(/\n$/,'').split('\n');
  return (
    <div style={{background:'var(--surface-sunken)',border:'var(--border-width) solid var(--border-subtle)',
      borderRadius:'var(--radius-md)',overflow:'hidden',...style}}>
      <div style={{overflow:'auto',maxHeight}}>
        <table style={{borderCollapse:'collapse',width:'100%',font:'var(--type-code)'}}>
          <tbody>
            {lines.map((l,i)=>(
              <tr key={i}>
                {showLineNumbers?<td style={{width:1,whiteSpace:'nowrap',textAlign:'right',padding:'0 12px 0 14px',
                  color:'var(--warm-400)',userSelect:'none',verticalAlign:'top'}}>{startLine+i}</td>:null}
                <td style={{padding:'0 14px 0 0',whiteSpace:'pre',color:'var(--code-punct)'}}>{paint(l)}</td>
              </tr>))}
          </tbody>
        </table>
      </div>
      {truncated?<div style={{textAlign:'center',padding:'4px 0 8px',color:'var(--text-muted)',font:'var(--type-meta)'}}>···</div>:null}
    </div>
  );
}
