const {Button,Checkbox,Badge,Icon,MetaList}=window.KalfaDesignSystem_c5b903;

function Clause({title,children}){return(
 <section style={{marginBottom:22}}>
  <h3 style={{font:'var(--fw-semibold) 17px/1.3 var(--font-serif)',color:'var(--text-title)',margin:'0 0 7px'}}>{title}</h3>
  <p style={{font:'var(--type-doc)',color:'var(--text-body)',margin:0,maxWidth:'62ch'}}>{children}</p>
 </section>);}

function OnboardingScreen({onAccept}){
 const [ok,setOk]=React.useState(false);
 return(
 <div className="kalfa-light" style={{flex:1,background:'var(--bg-app)',overflowY:'auto',display:'flex',justifyContent:'center',padding:'44px 28px'}}>
  <div style={{width:'min(760px,100%)'}}>
   <div style={{textAlign:'center',marginBottom:30}}>
    <div style={{font:'var(--type-wordmark)',letterSpacing:'var(--tracking-wordmark)',color:'var(--text-title)'}}>Kalfa</div>
    <div style={{font:'var(--type-tagline)',color:'var(--text-muted)',marginTop:6}}>Daha iyi yazılım, daha aydınlık yarınlar.</div>
   </div>
   <div style={{background:'var(--surface-card)',border:'1px solid var(--border-subtle)',borderRadius:'var(--radius-2xl)',boxShadow:'var(--shadow-paper)',padding:'32px 36px'}}>
    <div style={{display:'flex',alignItems:'center',gap:12,marginBottom:6}}>
     <h2 style={{font:'var(--type-display)',color:'var(--text-title)',margin:0}}>Kullanım koşulları</h2>
     <Badge tone="accent">Sürüm 3 · Önemli değişiklik</Badge>
    </div>
    <p style={{font:'var(--type-subtitle)',color:'var(--text-secondary)',margin:'0 0 24px'}}>Kalfa'yı kullanmadan önce lütfen okuyun. Kabul etmeden hiçbir içerik EVREN'e gönderilmez.</p>
    <div style={{height:1,background:'var(--border-subtle)',margin:'0 0 24px'}}/>
    <Clause title="Kalfa yerel çalışır">Kalfa makinenizde çalışır. Dosyalarınız, kabuk çıktılarınız ve proje içeriğiniz yalnızca siz onayladığınızda EVREN'e gönderilir. Onay, gösterilen işleme bağlıdır; kalıcı bir "her şeye izin ver" seçeneği yoktur.</Clause>
    <Clause title="Her gönderim kaydedilir">EVREN'e giden her istek, içeriği ve kaynaklarıyla birlikte gönderim günlüğüne yazılır. Günlüğü istediğiniz zaman inceleyebilir, dışa aktarabilir veya silebilirsiniz.</Clause>
    <Clause title="Turlar geri alınabilir">Dosya değişiklikleri yerel sürüm kontrolü ile geri alınabilir. Kabuk ve ağ etkileri geri alınamaz; Kalfa bu durumu onay ekranında açıkça belirtir.</Clause>
    <Clause title="Kabuk yalıtımı">Komutlar işletim sistemi düzeyinde yalıtılmış olarak çalışır — Linux'ta bubblewrap, Windows'ta ayrı kullanıcı — ve varsayılan olarak ağ erişimi yoktur.</Clause>
    <div style={{background:'var(--surface-sunken)',border:'1px solid var(--border-subtle)',borderRadius:'var(--radius-lg)',padding:'16px 18px',margin:'26px 0 22px'}}>
     <MetaList items={[{label:'Sürüm',value:'3',mono:true},{label:'Yürürlük',value:'24 Nis 2025',mono:true},{label:'Lisans',value:'Apache-2.0',mono:true},{label:'Sağlayıcı',value:'EVREN LLM Gateway'}]}/>
    </div>
    <div style={{display:'flex',alignItems:'center',gap:18,flexWrap:'wrap'}}>
     <Checkbox checked={ok} onChange={()=>setOk(!ok)} label="Koşulları okudum ve kabul ediyorum"/>
     <span style={{marginLeft:'auto',display:'flex',gap:12}}>
      <Button size="lg">Kabul etmiyorum</Button>
      <Button size="lg" variant="primary" disabled={!ok} onClick={onAccept}>Kabul ediyorum</Button>
     </span>
    </div>
   </div>
   <div style={{textAlign:'center',font:'var(--type-meta)',color:'var(--text-muted)',marginTop:22}}>Kalfa, dikkatli çalışır. Kontrol her zaman sizdedir.</div>
  </div>
 </div>);}
Object.assign(window,{OnboardingScreen});