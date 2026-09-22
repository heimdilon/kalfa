import React from 'react';
import {IconTile} from '../core/IconTile.jsx';

export function Message({author='kalfa',name,time,children,streaming,style}){
  const isUser=author==='user';
  return (
    <article style={{display:'flex',gap:12,padding:'14px 16px',
      background:isUser?'var(--surface-raised)':'transparent',
      border:isUser?'var(--border-width) solid var(--border-subtle)':'none',
      borderRadius:isUser?'var(--radius-xl)':0,...style}}>
      <IconTile icon={isUser?'user':'hexagon'} tone={isUser?'neutral':'accent'} size={32}/>
      <div style={{flex:1,minWidth:0}}>
        <header style={{display:'flex',alignItems:'baseline',gap:10,marginBottom:5}}>
          <span style={{font:'var(--fw-semibold) var(--text-small)/1 var(--font-sans)',color:'var(--text-title)'}}>
            {name||(isUser?'Sen':'Kalfa')}</span>
          {time?<span style={{font:'var(--type-meta)',color:'var(--text-muted)',marginLeft:'auto'}}>{time}</span>:null}
        </header>
        <div style={{font:'var(--type-body)',color:'var(--text-body)',textWrap:'pretty'}}>
          {children}
          {streaming?<span style={{display:'inline-block',width:8,height:15,marginLeft:2,verticalAlign:'-2px',
            background:'var(--copper-200)',animation:'kalfa-caret 1s steps(2) infinite'}}/>:null}
        </div>
      </div>
      <style>{'@keyframes kalfa-caret{50%{opacity:0}}'}</style>
    </article>
  );
}
