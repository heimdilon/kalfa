import React from 'react';
import {Icon} from '../core/Icon.jsx';
import {IconTile} from '../core/IconTile.jsx';

const KALFA_TOOL_STATUS={
  running:['loader-circle','var(--status-warn)'],
  done:['circle-check','var(--status-ok)'],
  denied:['circle-x','var(--status-danger)'],
  unknown:['circle-help','var(--text-muted)']
};

export function ToolCallCard({tool,target,meta,status='done',icon='file-text',children,style}){
  const [sIcon,sColor]=KALFA_TOOL_STATUS[status]||KALFA_TOOL_STATUS.done;
  return (
    <section style={{background:'var(--surface-card)',border:'var(--border-width) solid var(--border-subtle)',
      borderRadius:'var(--radius-xl)',overflow:'hidden',...style}}>
      <header style={{display:'flex',alignItems:'flex-start',gap:12,padding:'12px 14px'}}>
        <IconTile icon={icon} size={32}/>
        <div style={{flex:1,minWidth:0}}>
          <div style={{font:'var(--fw-semibold) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-title)'}}>{tool}</div>
          {target?<div style={{font:'var(--type-meta)',color:'var(--text-secondary)',marginTop:3}}>{target}</div>:null}
          {meta?<div style={{font:'var(--type-meta)',color:'var(--text-muted)',marginTop:6}}>{meta}</div>:null}
        </div>
        <Icon name={sIcon} size={19} color={sColor}/>
      </header>
      {children?<div style={{padding:'0 14px 14px'}}>{children}</div>:null}
    </section>
  );
}
