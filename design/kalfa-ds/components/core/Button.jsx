import React from 'react';
import {Icon} from './Icon.jsx';

const KALFA_BTN_VARIANTS={
  primary:{bg:'var(--action-primary)',fg:'var(--action-primary-fg)',bd:'transparent',hoverBg:'var(--action-primary-hover)',pressBg:'var(--action-primary-press)'},
  secondary:{bg:'transparent',fg:'var(--text-body)',bd:'var(--border-default)',hoverBg:'var(--surface-raised)',pressBg:'var(--surface-sunken)'},
  ghost:{bg:'transparent',fg:'var(--text-secondary)',bd:'transparent',hoverBg:'var(--surface-raised)',pressBg:'var(--surface-sunken)'},
  approve:{bg:'color-mix(in oklab,var(--green-400) 16%,transparent)',fg:'var(--status-ok)',bd:'color-mix(in oklab,var(--green-400) 45%,transparent)',hoverBg:'color-mix(in oklab,var(--green-400) 26%,transparent)',pressBg:'color-mix(in oklab,var(--green-400) 34%,transparent)'},
  caution:{bg:'color-mix(in oklab,var(--amber-200) 14%,transparent)',fg:'var(--status-warn)',bd:'color-mix(in oklab,var(--amber-200) 42%,transparent)',hoverBg:'color-mix(in oklab,var(--amber-200) 24%,transparent)',pressBg:'color-mix(in oklab,var(--amber-200) 32%,transparent)'},
  danger:{bg:'color-mix(in oklab,var(--red-400) 16%,transparent)',fg:'var(--status-danger)',bd:'color-mix(in oklab,var(--red-400) 45%,transparent)',hoverBg:'color-mix(in oklab,var(--red-400) 26%,transparent)',pressBg:'color-mix(in oklab,var(--red-400) 34%,transparent)'}
};
const KALFA_BTN_SIZES={sm:{h:28,px:10,font:'var(--text-micro)',icon:14,gap:6},md:{h:34,px:14,font:'var(--text-small)',icon:16,gap:8},lg:{h:42,px:20,font:'var(--text-body-size)',icon:16,gap:8}};

export function Button({variant='secondary',size='md',iconLeft,iconRight,fullWidth,disabled,loading,children,onClick,type='button',style,...rest}){
  const v=KALFA_BTN_VARIANTS[variant]||KALFA_BTN_VARIANTS.secondary;
  const s=KALFA_BTN_SIZES[size]||KALFA_BTN_SIZES.md;
  const [h,setH]=React.useState(false),[p,setP]=React.useState(false);
  const bg=disabled?'transparent':p?v.pressBg:h?v.hoverBg:v.bg;
  return (
    <button type={type} disabled={disabled||loading} onClick={onClick}
      onMouseEnter={()=>setH(true)} onMouseLeave={()=>{setH(false);setP(false);}}
      onMouseDown={()=>setP(true)} onMouseUp={()=>setP(false)}
      style={{display:'inline-flex',alignItems:'center',justifyContent:'center',gap:s.gap,
        height:s.h,padding:`0 ${s.px}px`,width:fullWidth?'100%':undefined,
        font:`var(--fw-medium) ${s.font}/1 var(--font-sans)`,letterSpacing:'var(--tracking-tight)',
        color:disabled?'var(--text-muted)':v.fg,background:bg,
        border:`var(--border-width) solid ${disabled?'var(--border-subtle)':v.bd}`,
        borderRadius:'var(--radius-md)',cursor:disabled?'not-allowed':'pointer',
        opacity:disabled?0.55:1,transition:'var(--transition-control)',
        transform:p&&!disabled?'translateY(0.5px)':'none',...style}} {...rest}>
      {loading?<Icon name="loader-circle" size={s.icon}/>:iconLeft?<Icon name={iconLeft} size={s.icon}/>:null}
      {children}
      {iconRight?<Icon name={iconRight} size={s.icon}/>:null}
    </button>
  );
}
