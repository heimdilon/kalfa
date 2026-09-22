import React from 'react';
import {Icon} from './Icon.jsx';

/** Square glyph tile used as the avatar for message authors and log rows. */
export function IconTile({icon,tone='neutral',size=32,style}){
  const map={neutral:['var(--text-secondary)','var(--surface-sunken)','var(--border-subtle)'],
    accent:['var(--text-accent)','var(--surface-selected)','color-mix(in oklab,var(--copper-400) 40%,transparent)'],
    ok:['var(--status-ok)','var(--green-tint)','color-mix(in oklab,var(--green-400) 35%,transparent)'],
    warn:['var(--status-warn)','var(--amber-tint)','color-mix(in oklab,var(--amber-200) 35%,transparent)'],
    danger:['var(--status-danger)','var(--red-tint)','color-mix(in oklab,var(--red-400) 35%,transparent)'],
    info:['var(--status-info)','var(--blue-tint)','color-mix(in oklab,var(--blue-200) 35%,transparent)']};
  const [fg,bg,bd]=map[tone]||map.neutral;
  return (
    <span style={{display:'inline-flex',alignItems:'center',justifyContent:'center',width:size,height:size,flex:'none',
      borderRadius:'var(--radius-md)',background:bg,color:fg,border:`var(--border-width) solid ${bd}`,...style}}>
      <Icon name={icon} size={Math.round(size*0.5)}/>
    </span>
  );
}
