const {PanelHeader,Select,Input,Switch,IconButton,Icon,Badge,LogTable,Pagination,SourceTag,TokenMeter,CodeBlock,Button,MetaList}=window.KalfaDesignSystem_c5b903;

const ROWS=[
 {id:'req_8f3a2c7e9d1b',time:'14:02:11',date:'24 Nis 2025',purpose:'Ana model',purposeNote:'Kod inceleme ve öneri',icon:'message-square',model:'glm-5.3',tokens:'2.341',sources:[['dosya','src/main.rs:1–120'],['mesaj','kullanıcı mesajı'],['gecmis','+3 önceki mesaj']]},
 {id:'req_6d9b1e0a3f2c',time:'13:58:03',date:'24 Nis 2025',purpose:'Eklenti: turkce-rag',purposeNote:'Bağlamsal arama',icon:'puzzle',model:'glm-5.3',tokens:'892',sources:[['kabuk','cargo test'],['dosya','Cargo.toml'],['mesaj','kullanıcı mesajı']]},
 {id:'req_4c2e9d7b1a8f',time:'13:54:27',date:'24 Nis 2025',purpose:'Özet',purposeNote:'Uzun bağlamı özetleme',icon:'file-text',model:'glm-5.3-mini',tokens:'1.120',sources:[['gecmis','+34 önceki mesaj'],['mesaj','sohbet geçmişi'],['dosya','README.md']]},
 {id:'req_9a7d3e2c6b1f',time:'13:48:10',date:'24 Nis 2025',purpose:'Denetçi',purposeNote:'Güvenlik ve politika kontrolü',icon:'shield',model:'glm-5.3',tokens:'756',sources:[['dosya','src/lib.rs:200–310'],['mesaj','güvenlik politikası']]},
 {id:'req_e1b4c6d9f0a3',time:'13:41:55',date:'24 Nis 2025',purpose:'Ana model',purposeNote:'Hata analizi',icon:'message-square',model:'glm-5.3',tokens:'3.012',sources:[['kabuk','cargo test'],['mesaj','hata çıktısı (stderr)'],['gecmis','+5 önceki mesaj']]},
 {id:'req_03f9a7d2c8e4',time:'13:37:22',date:'24 Nis 2025',purpose:'Eklenti: turkce-rag',purposeNote:'Dokümantasyon arama',icon:'puzzle',model:'glm-5.3',tokens:'645',sources:[['mesaj','sistem notları'],['dosya','web: docs.rs']]},
 {id:'req_7b2e1d9c4f6a',time:'13:30:18',date:'24 Nis 2025',purpose:'Özet',purposeNote:'Toplantı özeti',icon:'file-text',model:'glm-5.3-mini',tokens:'980',sources:[['gecmis','+12 önceki mesaj'],['dosya','notes.md']]},
 {id:'req_5d6c8b1a9e3f',time:'13:25:44',date:'24 Nis 2025',purpose:'Denetçi',purposeNote:'İçerik ve risk kontrolü',icon:'shield',model:'glm-5.3',tokens:'421',sources:[['mesaj','kurallar: güvenlik'],['gecmis','+1 önceki mesaj']]}];

const SENT="# Bağlam\nAşağıdaki Rust dosyalarının içeriğini ve\nkullanıcının sorusunu EVREN'e gönderiyorum.\n\n## Kullanıcı mesajı\nsrc/main.rs dosyasında hata yönetimini\niyileştirmek istiyorum.";

function LogRow({r,active,onSelect}){
 const [h,setH]=React.useState(false);
 return(
 <div onClick={()=>onSelect(r.id)} onMouseEnter={()=>setH(true)} onMouseLeave={()=>setH(false)}
  style={{display:'grid',gridTemplateColumns:'110px 1.3fr 130px 1fr 80px 1.5fr 28px',alignItems:'center',gap:14,padding:'12px 16px',cursor:'pointer',
   borderBottom:'1px solid var(--border-subtle)',background:active?'var(--surface-selected)':h?'var(--surface-raised)':'transparent',
   boxShadow:active?'var(--shadow-accent-edge)':'none',transition:'var(--transition-control)'}}>
  <div><div style={{font:'var(--type-meta)',color:'var(--text-body)'}}>{r.time}</div><div style={{font:'var(--type-meta)',fontSize:11,color:'var(--text-muted)'}}>{r.date}</div></div>
  <div style={{display:'flex',alignItems:'center',gap:10,minWidth:0}}>
   <span style={{width:28,height:28,flex:'0 0 auto',borderRadius:'var(--radius-md)',background:'var(--copper-tint)',display:'flex',alignItems:'center',justifyContent:'center'}}><Icon name={r.icon} size={15} color="var(--text-accent)"/></span>
   <span style={{minWidth:0}}><div style={{font:'var(--fw-medium) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-body)',overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{r.purpose}</div>
   <div style={{font:'var(--type-meta)',fontSize:11,color:'var(--text-muted)',overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{r.purposeNote}</div></span>
  </div>
  <div style={{font:'var(--type-meta)',color:'var(--text-secondary)'}}>{r.model}</div>
  <div style={{font:'var(--type-meta)',color:'var(--text-secondary)',overflow:'hidden',textOverflow:'ellipsis'}}>{r.id}</div>
  <div style={{font:'var(--type-meta)',color:'var(--text-body)'}}>{r.tokens}</div>
  <div style={{display:'flex',flexWrap:'wrap',gap:6}}>{r.sources.map(([k,v],i)=><SourceTag key={i} kind={k} value={v} live={k==='kabuk'} muted={k==='gecmis'}/>)}</div>
  <Icon name="chevron-right" size={16} color="var(--text-muted)"/>
 </div>);}

function LogDetail({r,onClose}){return(
 <aside style={{width:400,flex:'0 0 auto',display:'flex',flexDirection:'column',background:'var(--bg-panel)',borderLeft:'1px solid var(--border-subtle)',overflowY:'auto'}}>
  <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',padding:'18px 18px 8px'}}>
   <span style={{font:'var(--fw-semibold) 19px/1 var(--font-sans)',color:'var(--text-title)'}}>Gönderilen içerik</span>
   <IconButton icon="x" label="Kapat" onClick={onClose}/>
  </div>
  <div style={{padding:'0 18px 14px',display:'flex',alignItems:'center',gap:10}}>
   <span style={{font:'var(--fw-medium) 15px/1 var(--font-mono)',color:'var(--text-body)'}}>{r.id}</span>
   <Icon name="copy" size={15} color="var(--text-muted)"/>
   <span style={{marginLeft:'auto'}}><Badge tone="ok">Tamamlandı</Badge></span>
  </div>
  <div style={{padding:'0 18px 16px',font:'var(--type-meta)',color:'var(--text-muted)'}}>{r.date} {r.time} · {r.tokens} token · 12.4 sn</div>
  <div style={{padding:'0 18px 16px'}}>
   <div style={{font:'var(--type-label)',letterSpacing:'var(--tracking-label)',textTransform:'uppercase',color:'var(--text-muted)',marginBottom:8}}>Gönderim amacı</div>
   <div style={{display:'flex',alignItems:'center',gap:10}}>
    <span style={{width:32,height:32,borderRadius:'var(--radius-md)',background:'var(--copper-tint)',display:'flex',alignItems:'center',justifyContent:'center'}}><Icon name={r.icon} size={16} color="var(--text-accent)"/></span>
    <span><div style={{font:'var(--fw-medium) var(--text-small)/1.3 var(--font-sans)',color:'var(--text-body)'}}>{r.purpose}</div><div style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>{r.purposeNote}</div></span>
   </div>
  </div>
  <div style={{padding:'0 18px 16px'}}>
   <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',marginBottom:8}}>
    <span style={{font:'var(--type-label)',letterSpacing:'var(--tracking-label)',textTransform:'uppercase',color:'var(--text-muted)'}}>Gönderilen içerik</span>
    <span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>Biçim: Markdown</span>
   </div>
   <CodeBlock startLine={1} code={SENT}/>
  </div>
  <div style={{padding:'0 18px 24px'}}>
   <div style={{font:'var(--type-label)',letterSpacing:'var(--tracking-label)',textTransform:'uppercase',color:'var(--text-muted)',marginBottom:10}}>Token kullanımı</div>
   <TokenMeter total={4096} used={2341} segments={[{label:'Sistem',value:312,color:'var(--status-info)'},{label:'Kullanıcı',value:1842,color:'var(--copper-400)'},{label:'Araçlar',value:187,color:'var(--status-warn)'}]}/>
  </div>
 </aside>);}

function LogScreen(){
 const [sel,setSel]=React.useState('req_8f3a2c7e9d1b');
 const [page,setPage]=React.useState(1);
 const [pre,setPre]=React.useState(true);
 const row=ROWS.find(r=>r.id===sel);
 return(
 <div style={{flex:1,display:'flex',minWidth:0}}>
  <div style={{flex:1,display:'flex',flexDirection:'column',minWidth:0,background:'var(--bg-app)'}}>
   <div style={{padding:'20px 22px 14px',display:'flex',alignItems:'flex-start',gap:16}}>
    <div><div style={{font:'var(--fw-semibold) 24px/1.2 var(--font-sans)',color:'var(--text-title)',letterSpacing:'var(--tracking-tight)'}}>Gönderim günlüğü</div>
    <div style={{font:'var(--type-small)',color:'var(--text-muted)',marginTop:5}}>EVREN'e gönderilen tüm istekler, içerikleri ve kaynakları</div></div>
    <div style={{marginLeft:'auto',display:'flex',alignItems:'center',gap:12}}>
     <div style={{width:250}}><Switch checked={pre} onChange={()=>setPre(!pre)} label="Göndermeden önce göster"/></div>
     <IconButton icon="more-horizontal" label="Daha fazla"/>
    </div>
   </div>
   <div style={{display:'flex',gap:12,padding:'0 22px 14px',alignItems:'center'}}>
    <Select icon="calendar" value="Son 24 saat" options={['Son 1 saat','Son 24 saat','Son 7 gün','Tümü']}/>
    <Select icon="filter" value="Tüm amaçlar" options={['Tüm amaçlar','Ana model','Özet','Denetçi','Eklenti']}/>
    <div style={{flex:1}}><Input iconLeft="search" placeholder="İstek kimliği, kaynak veya içerikte ara…"/></div>
    <IconButton icon="refresh-cw" label="Yenile"/>
   </div>
   <div style={{flex:1,overflowY:'auto',borderTop:'1px solid var(--border-subtle)'}}>
    <div style={{display:'grid',gridTemplateColumns:'110px 1.3fr 130px 1fr 80px 1.5fr 28px',gap:14,padding:'10px 16px',
      font:'var(--type-label)',letterSpacing:'var(--tracking-label)',textTransform:'uppercase',color:'var(--text-muted)',
      background:'var(--surface-sunken)',borderBottom:'1px solid var(--border-subtle)',position:'sticky',top:0,zIndex:1}}>
     <span>Zaman</span><span>Amaç</span><span>Model</span><span>İstek kimliği</span><span>Token</span><span>İlk kez gönderilen</span><span/>
    </div>
    {ROWS.map(r=><LogRow key={r.id} r={r} active={r.id===sel} onSelect={setSel}/>)}
   </div>
   <div style={{display:'flex',alignItems:'center',gap:16,padding:'12px 22px',borderTop:'1px solid var(--border-subtle)'}}>
    <span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>1–10 / 42 istek</span>
    <span style={{margin:'0 auto'}}><Pagination page={page} pages={5} onChange={setPage}/></span>
    <span style={{display:'flex',alignItems:'center',gap:10}}><span style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>Her sayfada</span><Select value="10" options={['10','25','50']}/></span>
   </div>
  </div>
  {row?<LogDetail r={row} onClose={()=>setSel(null)}/>:null}
 </div>);}
Object.assign(window,{LogScreen});