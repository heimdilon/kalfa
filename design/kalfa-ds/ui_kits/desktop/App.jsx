function App(){
 const [accepted,setAccepted]=React.useState(false);
 const [route,setRoute]=React.useState('oturumlar');
 const [session,setSession]=React.useState('s1');
 const [busy,setBusy]=React.useState(false);
 const [sheet,setSheet]=React.useState(false);
 if(!accepted)return <div className="kalfa-root" style={{position:'absolute',inset:0,display:'flex'}}><OnboardingScreen onAccept={()=>setAccepted(true)}/></div>;
 return(
 <div className="kalfa-root" style={{position:'absolute',inset:0,display:'flex',flexDirection:'column',overflow:'hidden'}}>
  <div style={{flex:1,display:'flex',minHeight:0,position:'relative'}}>
   <Rail route={route} go={setRoute}/>
   {route==='gunluk'?<LogScreen/>:<>
    <SessionList active={session} onPick={setSession}/>
    <WorkspaceScreen busy={busy} onApprove={()=>setSheet(true)}
      onSend={()=>{setBusy(true);setTimeout(()=>{setBusy(false);setSheet(true);},900);}}/>
   </>}
   {sheet?<ApprovalSheet onClose={()=>setSheet(false)}/>:null}
  </div>
  <AppStatusBar busy={busy}/>
 </div>);}
ReactDOM.createRoot(document.getElementById('root')).render(<App/>);