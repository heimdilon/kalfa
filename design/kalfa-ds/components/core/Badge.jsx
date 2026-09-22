import React from 'react';
import {Icon} from './Icon.jsx';

const KALFA_BADGE_TONES={
  neutral:['var(--text-secondary)','color-mix(in oklab,var(--warm-200) 10%,transparent)','color-mix(in oklab,var(--warm-200) 22%,transparent)'],
  accent:['var(--text-accent)','var(--surface-selected)','color-mix(in oklab,var(--copper-400) 45%,transparent)'],
  ok:['var(--status-ok)','color-mix(in oklab,var(--green-400) 14%,transparent)','color-mix(in oklab,var(--green-400) 38%,transparent)'],
  warn:['var(--status-warn)','color-mix(in oklab,var(--amber-200) 14%,transparent)','color-mix(in oklab,var(--amber-200) 38%,transparent)'],
  danger:['var(--status-danger)','color-mix(in oklab,var(--red-400) 14%,transparent)','color-mix(in oklab,var(--red-400) 38%,transparent)'],
  info:['var(--status-info)','color-mix(in oklab,var(--blue-200) 14%,transparent)','color-mix(in oklab,var(--blue-200) 38%,transparent)']
};

export function Badge({tone='neutral',icon,dot,shape='pill',children,style}){
  const [fg,bg,bd]=KALFA_BADGE_TONES[tone]||KALFA_BADGE_TONES.neutral;
  return (
    <span style={{display:'inline-flex',alignItems:'center',gap:6,height:22,padding:'0 9px',
      borderRadius:shape==='pill'?'var(--radius-pill)':'var(--radius-sm)',
      font:'var(--fw-medium) var(--text-micro)/1 var(--font-sans)',color:fg,background:bg,
      border:`var(--border-width) solid ${bd}`,whiteSpace:'nowrap',...style}}>
      {dot?<span style={{width:6,height:6,borderRadius:'50%',background:fg,flex:'none'}}/>:null}
      {icon?<Icon name={icon} size={12}/>:null}
      {children}
    </span>
  );
}
