const {PanelHeader,Message,ThoughtPanel,ToolCallCard,TurnSummary,Composer,CodeBlock,FileTree,MetaList,Button,IconButton,Icon,Badge}=window.KalfaDesignSystem_c5b903;

const SRC="import { verify, JwtPayload } from 'jsonwebtoken'\nimport { NextFunction, Request, Response } from 'express'\nimport { getUserById } from '../services/user'\n\nexport async function requireAuth(req: Request, res: Response, next: NextFunction) {\n  try {\n    const token = extractToken(req)\n    if (!token) {";

const TREE=[
 {name:'src',type:'dir',open:true,depth:0},
 {name:'controllers',type:'dir',depth:1},
 {name:'middleware',type:'dir',open:true,depth:1},
 {name:'auth.ts',path:'src/middleware/auth.ts',depth:2},
 {name:'rateLimit.ts',depth:2},
 {name:'errorHandler.ts',depth:2},
 {name:'models',type:'dir',depth:1},
 {name:'routes',type:'dir',depth:1},
 {name:'services',type:'dir',depth:1},
 {name:'utils',type:'dir',depth:1},
 {name:'tests',type:'dir',depth:0},
 {name:'docs',type:'dir',depth:0},
 {name:'.env.example',depth:0},
 {name:'package.json',depth:0},
 {name:'tsconfig.json',depth:0},
 {name:'README.md',depth:0}];

function ConvHeader({onApprove}){return(
 <header style={{display:'flex',alignItems:'center',gap:16,padding:'14px 22px',borderBottom:'1px solid var(--border-subtle)',flex:'0 0 auto'}}>
  <div style={{minWidth:0}}>
   <div style={{font:'var(--fw-semibold) 20px/1.2 var(--font-sans)',color:'var(--text-title)',letterSpacing:'var(--tracking-tight)'}}>Kullanıcı yönetimi API</div>
   <div style={{font:'var(--type-small)',color:'var(--text-muted)',marginTop:3}}>JWT doğrulama ve yetkilendirme</div>
  </div>
  <div style={{marginLeft:'auto',display:'flex',alignItems:'center',gap:14}}>
   <span style={{display:'inline-flex',alignItems:'center',gap:7,font:'var(--type-meta)',color:'var(--text-secondary)'}}><Icon name="folder" size={15} color="var(--text-muted)"/>~/projeler/kalfa-api</span>
   <span style={{display:'inline-flex',alignItems:'center',gap:7,font:'var(--type-meta)',color:'var(--text-secondary)'}}><Icon name="git-branch" size={15} color="var(--text-muted)"/>main <Icon name="chevron-down" size={14} color="var(--text-muted)"/></span>
   <Button iconLeft="share-2" onClick={onApprove}>Paylaş</Button>
   <IconButton icon="more-horizontal" label="Daha fazla"/>
  </div>
 </header>);}

function Conversation({onSend,busy}){return(
 <div style={{flex:1,overflowY:'auto',padding:'20px 22px',display:'flex',flexDirection:'column',gap:14}}>
  <Message author="user" time="14:28">Kullanıcı kimlik doğrulama akışını gözden geçirip olası güvenlik açıklarını kontrol eder misin? Özellikle JWT kullanımı, refresh token mantığı ve yetkilendirme middleware'ini incele.</Message>
  <Message time="14:29">Elbette. Kimlik doğrulama akışını inceledim. Genel yapı sağlam, ancak bazı iyileştirme alanları var. Önce ilgili dosyaları okuyup akışı analiz ettim, ardından bulgularımı aşağıda özetledim.</Message>
  <ThoughtPanel steps={3}>Önce ilgili dosyaları listeledim, ardından auth.ts ve rateLimit.ts dosyalarını okudum ve token yaşam döngüsünü izledim.</ThoughtPanel>
  <ToolCallCard tool="Dosya okundu" target="src/middleware/auth.ts" meta="412 satır · 18 KB" status="done">
   <CodeBlock startLine={1} truncated code={SRC}/>
  </ToolCallCard>
  <TurnSummary time="14:30" onUndo={()=>{}} items={[
   'JWT doğrulama mantığı doğru, ancak token süresi hatalı yapılandırılmış (24 saat yerine 7 gün).',
   "Refresh token'lar veritabanında hash'lenmeden saklanıyor.",
   "Yetkilendirme middleware'inde bazı rotalar için rol kontrolü eksik.",
   'Küçük iyileştirmelerle güvenlik yükseltilebilir.']}/>
  {busy?<Message time="14:31" streaming>Rol kontrolü eksik olan rotaları listeliyorum</Message>:null}
 </div>);}

function ProjectPanel(){return(
 <aside style={{width:'var(--inspector-width)',flex:'0 0 auto',display:'flex',flexDirection:'column',background:'var(--bg-panel)',borderLeft:'1px solid var(--border-subtle)'}}>
  <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',padding:'18px 18px 10px'}}>
   <span style={{font:'var(--type-title)',color:'var(--text-title)'}}>Proje dosyaları</span>
   <Icon name="chevron-right" size={16} color="var(--text-muted)"/>
  </div>
  <div style={{padding:'0 14px 8px'}}>
   <span style={{display:'inline-flex',alignItems:'center',gap:8,font:'var(--type-meta)',color:'var(--text-secondary)'}}><Icon name="folder-open" size={15} color="var(--text-muted)"/>~/projeler/kalfa-api</span>
  </div>
  <div style={{flex:1,overflowY:'auto',padding:'0 10px 12px'}}><FileTree activePath="src/middleware/auth.ts" items={TREE}/></div>
  <div style={{borderTop:'1px solid var(--border-subtle)',padding:'16px 18px'}}>
   <div style={{font:'var(--type-title)',color:'var(--text-title)',marginBottom:8}}>Dosya önizleme</div>
   <div style={{font:'var(--type-meta)',color:'var(--text-secondary)',marginBottom:12}}>src/middleware/auth.ts</div>
   <MetaList items={[{label:'Tür',value:'TypeScript'},{label:'Satır',value:'412',mono:true},{label:'Boyut',value:'18 KB',mono:true},{label:'Son değişiklik',value:'24 Nis 2025, 10:14',mono:true}]}/>
  </div>
 </aside>);}

function WorkspaceScreen({onSend,busy,onApprove}){return(
 <div style={{flex:1,display:'flex',minWidth:0}}>
  <div style={{flex:1,display:'flex',flexDirection:'column',minWidth:0,background:'var(--bg-app)'}}>
   <ConvHeader onApprove={onApprove}/>
   <Conversation busy={busy}/>
   <div style={{padding:'0 22px 18px'}}><Composer model="glm-5.3" mode="Sıkı" onSend={onSend}/></div>
  </div>
  <ProjectPanel/>
 </div>);}
Object.assign(window,{WorkspaceScreen});