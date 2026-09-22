import React from 'react';

function pascal(name){return String(name).split(/[-_\s]+/).filter(Boolean).map(s=>s[0].toUpperCase()+s.slice(1)).join('');}

/** Thin wrapper over the Lucide CDN set. Kalfa draws no custom glyphs. */
export function Icon({name,size=16,strokeWidth=1.75,color='currentColor',className,style,title}){
  const [,force]=React.useState(0);
  React.useEffect(()=>{
    if(typeof window==='undefined'||window.lucide)return;
    let n=0;const t=setInterval(()=>{if(window.lucide||++n>60){clearInterval(t);force(v=>v+1);}},50);
    return()=>clearInterval(t);
  },[]);
  const L=typeof window!=='undefined'?window.lucide:null;
  const node=L&&L.icons?(L.icons[pascal(name)]||L.icons[name]):null;
  const base={width:size,height:size,display:'inline-block',flex:'none',verticalAlign:'middle',...style};
  if(!node)return React.createElement('span',{className,style:base,'aria-hidden':true});
  // Lucide ships either a flat child list ([['path',{…}],…]) or a full node
  // triple (['svg',{…},[children]]) depending on build — normalise both.
  let list=Array.isArray(node)?node:[];
  if(typeof list[0]==='string'&&Array.isArray(list[2]))list=list[2];
  const kids=list.map((c,i)=>{
    if(!Array.isArray(c)||typeof c[0]!=='string')return null;
    const [tag,attrs]=c;return React.createElement(tag,{key:i,...attrs});
  }).filter(Boolean);
  return React.createElement('svg',{
    className,role:title?'img':'presentation','aria-hidden':title?undefined:true,
    xmlns:'http://www.w3.org/2000/svg',viewBox:'0 0 24 24',fill:'none',
    stroke:color,strokeWidth,strokeLinecap:'round',strokeLinejoin:'round',style:base
  },title?[React.createElement('title',{key:'t'},title),...kids]:kids);
}
