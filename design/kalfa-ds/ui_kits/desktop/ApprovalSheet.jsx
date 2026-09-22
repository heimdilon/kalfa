const {Button,Checkbox,Select,Textarea,Icon,DiffView,IconTile,Badge}=window.KalfaDesignSystem_c5b903;

const FILES=[{path:'src/main.rs',add:12,del:4},{path:'src/izin.rs',add:28,del:6},{path:'tests/izin.rs',add:14,del:2}];
const DIFF=[
 {old:42,neu:42,text:'        let config = Config::load()?;'},
 {old:43,neu:43,text:'        let mut app = App::new(config);'},
 {old:44,neu:44,text:''},
 {kind:'del',old:45,text:'        // doğrudan dosya sistemi erişimi'},
 {kind:'del',old:46,text:'        app.fs = Fs::new();'},
 {kind:'del',old:47,text:'        app.run().await'},
 {kind:'add',neu:48,text:'        // izinli dosya sistemi erişimi'},
 {kind:'add',neu:49,text:'        app.fs = Fs::with_permission_manager(izin::PermissionManager::new()?);'},
 {kind:'add',neu:50,text:''},
 {kind:'add',neu:52,text:'        if app.config().sandbox {'},
 {kind:'add',neu:53,text:'            tracing::info!("yalıtımlı mod etkin");'},
 {kind:'add',neu:55,text:'        }'},
 {old:57,neu:57,text:'        app.run().await'}];

function Fact({icon,label,children}){return(
 <div style={{display:'grid',gridTemplateColumns:'20px 110px 1fr',gap:12,alignItems:'start'}}>
  <Icon name={icon} size={17} color="var(--text-muted)"/>
  <span style={{font:'var(--fw-medium) var(--text-small)/1.5 var(--font-sans)',color:'var(--text-body)'}}>{label}</span>
  <span style={{font:'var(--type-small)',color:'var(--text-secondary)',lineHeight:1.55}}>{children}</span>
 </div>);}

function ApprovalSheet({onClose}){
 const [sel,setSel]=React.useState(['src/main.rs','src/izin.rs','tests/izin.rs']);
 const [open,setOpen]=React.useState('src/main.rs');
 const toggle=p=>setSel(s=>s.includes(p)?s.filter(x=>x!==p):[...s,p]);
 return(
 <div onClick={onClose} style={{position:'absolute',inset:0,background:'var(--scrim)',backdropFilter:'var(--blur-scrim)',display:'flex',alignItems:'center',justifyContent:'center',padding:28,zIndex:20}}>
  <div onClick={e=>e.stopPropagation()} style={{width:'min(1140px,100%)',maxHeight:'100%',overflowY:'auto',background:'var(--bg-panel)',border:'1px solid var(--border-default)',borderRadius:'var(--radius-2xl)',boxShadow:'var(--shadow-overlay)',padding:'26px 28px'}}>
   <div style={{display:'flex',gap:18,alignItems:'flex-start',marginBottom:22}}>
    <IconTile icon="shield-check" tone="accent" size={56}/>
    <div><div style={{font:'var(--fw-semibold) 23px/1.3 var(--font-sans)',color:'var(--text-title)',letterSpacing:'var(--tracking-tight)'}}>Kalfa 3 dosyada değişiklik yapmak istiyor. Bu değişiklikler geri alınabilir.</div>
    <div style={{font:'var(--type-subtitle)',color:'var(--text-muted)',marginTop:6}}>Devam etmeden önce lütfen aşağıdaki değişiklikleri, etkisini ve kapsamını inceleyin.</div></div>
   </div>
   <div style={{display:'grid',gridTemplateColumns:'340px 1fr',gap:18,marginBottom:20}}>
    <div style={{background:'var(--surface-sunken)',border:'1px solid var(--border-subtle)',borderRadius:'var(--radius-lg)',padding:10}}>
     <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',padding:'6px 8px 12px',borderBottom:'1px solid var(--border-subtle)',marginBottom:8}}>
      <Checkbox checked={sel.length===3} indeterminate={sel.length>0&&sel.length<3} label="3 dosya" onChange={()=>setSel(sel.length===3?[]:FILES.map(f=>f.path))}/>
      <Checkbox checked={sel.length===3} label="Tümünü seç" onChange={()=>setSel(sel.length===3?[]:FILES.map(f=>f.path))}/>
     </div>
     {FILES.map(f=>{const active=open===f.path;return(
      <div key={f.path} onClick={()=>setOpen(f.path)} style={{display:'flex',alignItems:'center',gap:10,padding:'10px 10px',cursor:'pointer',borderRadius:'var(--radius-md)',
        background:active?'var(--surface-selected)':'transparent',boxShadow:active?'var(--shadow-accent-edge)':'none',transition:'var(--transition-control)'}}>
       <span onClick={e=>{e.stopPropagation();toggle(f.path);}}><Checkbox checked={sel.includes(f.path)} onChange={()=>{}}/></span>
       <Icon name="file-text" size={16} color="var(--text-muted)"/>
       <span style={{flex:1,font:'var(--fw-medium) var(--text-small)/1 var(--font-mono)',color:'var(--text-body)'}}>{f.path}</span>
       <span style={{font:'var(--type-meta)',color:'var(--status-ok)'}}>+{f.add}</span>
       <span style={{font:'var(--type-meta)',color:'var(--status-danger)'}}>−{f.del}</span>
       <Icon name="chevron-right" size={15} color="var(--text-muted)"/>
      </div>);})}
    </div>
    <div style={{background:'var(--surface-sunken)',border:'1px solid var(--border-subtle)',borderRadius:'var(--radius-lg)',overflow:'hidden'}}>
     <div style={{display:'flex',alignItems:'center',gap:12,padding:'12px 16px',borderBottom:'1px solid var(--border-subtle)'}}>
      <Icon name="file-text" size={17} color="var(--text-muted)"/>
      <div style={{flex:1,minWidth:0}}><div style={{font:'var(--fw-medium) var(--text-small)/1.3 var(--font-mono)',color:'var(--text-body)'}}>{open}</div>
      <div style={{font:'var(--type-meta)',color:'var(--text-muted)'}}>/home/kullanici/projeler/kalfa/{open}</div></div>
      <span style={{font:'var(--type-meta)',color:'var(--status-ok)'}}>+12</span><span style={{font:'var(--type-meta)',color:'var(--status-danger)'}}>−4</span>
      <Icon name="chevron-down" size={16} color="var(--text-muted)"/>
     </div>
     <DiffView hunkHeader="@@ -42,10 +42,18 @@ async fn main() -> Result<()> {" lines={DIFF}/>
    </div>
   </div>
   <div style={{display:'grid',gridTemplateColumns:'1fr 1fr',gap:28,padding:'20px 0',borderTop:'1px solid var(--border-subtle)',borderBottom:'1px solid var(--border-subtle)',marginBottom:20}}>
    <div style={{display:'flex',flexDirection:'column',gap:14}}>
     <Fact icon="settings" label="Ne">Kalfa, dosya sistemi izin yönetimini eklemek ve ilgili testleri güncellemek için 3 dosyada değişiklik yapmak istiyor.</Fact>
     <Fact icon="map-pin" label="Nerede"><span style={{fontFamily:'var(--font-mono)'}}>/home/kullanici/projeler/kalfa</span></Fact>
     <Fact icon="zap" label="Etki">Uygulamanın dosya erişim davranışını değiştirir. İzin kontrollü etkinleştirilir, testler güncellenir.</Fact>
     <Fact icon="upload" label="Veri çıkışı">Bu işlem herhangi bir harici veri göndermez.</Fact>
    </div>
    <div style={{display:'flex',flexDirection:'column',gap:14}}>
     <Fact icon="shield" label="Yalıtım">Değişiklikler mevcut proje dizini ile sınırlıdır. Harici sistemlere erişim yapılmayacaktır.</Fact>
     <Fact icon="rotate-ccw" label="Geri alınabilir mi">Evet. Değişiklikler yerel sürüm kontrolü ile geri alınabilir.</Fact>
     <Fact icon="target" label="Kapsam">Bu iznin hangi çağrılar için geçerli olacağını seçin.
      <div style={{marginTop:10}}><Select value="Yalnızca bu çağrı" options={['Yalnızca bu çağrı','Bu oturum boyunca','Bu proje için']}/></div>
     </Fact>
    </div>
   </div>
   <div style={{display:'grid',gridTemplateColumns:'1fr auto',gap:22,alignItems:'flex-end'}}>
    <div><div style={{font:'var(--type-small)',color:'var(--text-secondary)',marginBottom:8}}>Reddetme nedeni (isteğe bağlı)</div>
    <Textarea rows={2} placeholder="Reddetme nedeninizi buraya yazabilirsiniz…"/></div>
    <div style={{display:'flex',gap:12}}>
     <Button variant="approve" size="lg" iconLeft="check" onClick={onClose}>Tümünü onayla</Button>
     <Button variant="caution" size="lg" iconLeft="check" onClick={onClose}>Seçilenleri onayla</Button>
     <Button variant="danger" size="lg" iconLeft="x" onClick={onClose}>Reddet</Button>
    </div>
   </div>
  </div>
 </div>);}
Object.assign(window,{ApprovalSheet});