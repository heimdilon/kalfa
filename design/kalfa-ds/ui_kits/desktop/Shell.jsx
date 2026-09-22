const {NavRail,NavSection,NavItem,SessionRow,PanelHeader,StatusBar,StatusItem,Input,IconButton,Icon,Button}=window.KalfaDesignSystem_c5b903;

function Wordmark(){return(
 <div>
  <div style={{font:'var(--type-wordmark)',letterSpacing:'var(--tracking-wordmark)',color:'var(--text-title)'}}>Kalfa</div>
  <div style={{font:'var(--type-tagline)',color:'var(--text-muted)',marginTop:4,lineHeight:1.45}}>Daha iyi yazılım,<br/>daha aydınlık yarınlar.</div>
 </div>);}

function RailFooter(){return(
 <div>
  <div style={{width:34,height:2,background:'var(--copper-400)',marginBottom:10}}/>
  <div style={{font:'var(--type-small)',color:'var(--text-muted)',lineHeight:1.45}}>Küçük adımlar,<br/>büyük işler.</div>
 </div>);}

const NAV=[['oturumlar','layout-panel-left','Oturumlar'],['calisma','folder-tree','Çalışma alanı'],['eklentiler','puzzle','Eklentiler'],['gunluk','scroll-text','Gönderim günlüğü']];

function Rail({route,go}){return(
 <NavRail header={<Wordmark/>} footer={<RailFooter/>} style={{paddingTop:8}}>
  {NAV.map(([id,icon,label])=><NavItem key={id} icon={icon} active={route===id} onClick={()=>go(id)} style={{height:44,borderRadius:0,marginLeft:-8,marginRight:-8,paddingLeft:18}}>{label}</NavItem>)}
 </NavRail>);}

const SESSIONS=[
 {group:'Bugün',items:[
  {id:'s1',title:'Kullanıcı yönetimi API',preview:'JWT doğrulama ve yetkilendirme',time:'14:28'},
  {id:'s2',title:'Veritabanı şema düzeni',preview:'PostgreSQL tabloları',time:'11:03'},
  {id:'s3',title:'Ön yüz bileşen iyileştirme',preview:'Tarih seçici ve tablo',time:'09:41'}]},
 {group:'Dün',items:[
  {id:'s4',title:'Hata analizi',preview:'500 hatası incelemesi',time:'18:27'},
  {id:'s5',title:'Eklenti geliştirme',preview:'VS Code uyumlu araç',time:'15:12'},
  {id:'s6',title:'Dokümantasyon',preview:'Kurulum ve kullanım',time:'10:06'}]},
 {group:'Geçen hafta',items:[
  {id:'s7',title:'Test altyapısı',preview:'Birim ve bütünleşik testler',time:'Cmt'},
  {id:'s8',title:'Performans iyileştirme',preview:'Sorgu optimizasyonu',time:'Cum'},
  {id:'s9',title:'Dağıtım süreci',preview:'Tauri v2 paketleme',time:'24 Nis'},
  {id:'s10',title:'Proje planı',preview:'v1.0 dönüm noktaları',time:'22 Nis'}]}];

function SessionList({active,onPick}){
 const [q,setQ]=React.useState('');
 return(
 <aside style={{width:'var(--list-width)',flex:'0 0 auto',display:'flex',flexDirection:'column',background:'var(--bg-panel)',borderRight:'1px solid var(--border-subtle)'}}>
  <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',padding:'16px 18px 10px'}}>
   <span style={{font:'var(--fw-semibold) 19px/1 var(--font-sans)',color:'var(--text-title)'}}>Oturumlar</span>
   <IconButton icon="plus" label="Yeni oturum"/>
  </div>
  <div style={{padding:'0 16px 12px'}}><Input iconLeft="search" placeholder="Oturumlarda ara…" value={q} onChange={e=>setQ(e.target.value)}/></div>
  <div style={{flex:1,overflowY:'auto',padding:'0 10px 12px'}}>
   {SESSIONS.map(g=>{
    const items=g.items.filter(i=>i.title.toLowerCase().includes(q.toLowerCase()));
    if(!items.length)return null;
    return <div key={g.group} style={{marginBottom:14}}>
     <div style={{font:'var(--type-small)',color:'var(--text-muted)',padding:'10px 8px 6px'}}>{g.group}</div>
     <div style={{display:'flex',flexDirection:'column'}}>
      {items.map(i=><SessionRow key={i.id} active={active===i.id} title={i.title} preview={i.preview} time={i.time} onClick={()=>onPick(i.id)}
        style={{borderBottom:'1px solid var(--border-subtle)',borderRadius:active===i.id?'var(--radius-md)':0,padding:'12px 10px'}}/>)}
     </div></div>;})}
  </div>
 </aside>);}

function AppStatusBar({busy}){return(
 <StatusBar left={<StatusItem icon="circle" tone={busy?'warn':'ok'}>{busy?'Çalışıyor':'Hazır'}</StatusItem>}
  right={<>
   <StatusItem icon="database">1000 CR</StatusItem>
   <StatusItem>12,4k token</StatusItem>
   <StatusItem>%18 bağlam</StatusItem>
   <StatusItem icon="shield">Sıkı</StatusItem>
   <StatusItem icon="square-terminal">Kabuk yalıtımlı</StatusItem>
  </>}/>);}

Object.assign(window,{Wordmark,Rail,SessionList,AppStatusBar,SESSIONS});