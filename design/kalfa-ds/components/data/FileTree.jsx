import React from 'react';
import {Icon} from '../core/Icon.jsx';

export function FileTree({items=[],activePath,onSelect,style}){
  return (
    <div style={{display:'flex',flexDirection:'column',gap:1,...style}}>
      {items.map((it,i)=>{
        const active=it.path&&it.path===activePath;
        const isDir=it.type==='dir';
        return (
          <FileTreeRow key={it.path||i} item={it} depth={it.depth||0} active={active}
            isDir={isDir} onSelect={onSelect}/>
        );
      })}
    </div>
  );
}

function FileTreeRow({item,depth,active,isDir,onSelect}){
  const [h,setH]=React.useState(false);
  return (
    <div onClick={()=>onSelect&&onSelect(item)}
      onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
      style={{display:'flex',alignItems:'center',gap:7,height:26,cursor:'pointer',
        paddingLeft:10+depth*16,paddingRight:10,borderRadius:'var(--radius-sm)',
        background:active?'var(--surface-selected-strong)':h?'var(--surface-raised)':'transparent',
        color:active?'var(--text-accent)':'var(--text-secondary)',
        font:'var(--type-small)',transition:'var(--transition-control)'}}>
      {isDir?<Icon name={item.open?'chevron-down':'chevron-right'} size={13} color="var(--text-muted)"/>
        :<span style={{width:13,flex:'none'}}/>}
      <Icon name={isDir?(item.open?'folder-open':'folder'):'file'} size={14}
        color={active?'var(--text-accent)':isDir?'var(--copper-400)':'var(--text-muted)'}/>
      <span style={{overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{item.name}</span>
    </div>
  );
}
