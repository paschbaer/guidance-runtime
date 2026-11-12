#[cfg(not(windows))] fn main(){eprintln!("guidance-tray is available on Windows only");}
#[cfg(windows)] fn main()->anyhow::Result<()> {
    use guidance_protocol::Request;
    use guidance_update::UpdateConfig;
    use semver::Version;
    use serde_json::json;
    use std::path::Path;
    use tray_icon::{menu::{Menu,MenuEvent,MenuItem,PredefinedMenuItem},TrayIconBuilder};
    use winit::{event::Event,event_loop::{ControlFlow,EventLoop}};

    let event_loop=EventLoop::new()?; event_loop.set_control_flow(ControlFlow::Wait); let menu=Menu::new();
    let status=MenuItem::new("Guidance Runtime: connecting",false,None);
    let channel_text=UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG)).map(|c|format!("Update channel: {}",c.channel)).unwrap_or_else(|_|"Update channel: not configured".into());
    let channel=MenuItem::new(channel_text,false,None); let check=MenuItem::new("Check for updates",true,None);
    let refresh=MenuItem::new("Refresh",true,None); let start=MenuItem::new("Start All Enabled",true,None); let stop=MenuItem::new("Stop All",true,None); let exit=MenuItem::new("Exit Tray",true,None);
    menu.append_items(&[&status,&channel,&check,&PredefinedMenuItem::separator(),&start,&stop,&refresh,&PredefinedMenuItem::separator(),&exit])?;
    let _tray=TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("Guidance Runtime").build()?;
    let runtime=tokio::runtime::Runtime::new()?; let(refresh_id,start_id,stop_id,check_id,exit_id)=(refresh.id().clone(),start.id().clone(),stop.id().clone(),check.id().clone(),exit.id().clone());
    event_loop.run(move|event,elwt| if let Event::AboutToWait=event { if let Ok(item)=MenuEvent::receiver().try_recv() {
        if item.id==exit_id { elwt.exit(); }
        else if item.id==refresh_id { let ok=runtime.block_on(guidance_ipc::request(Request::new("runtime.status",json!({})))).map(|r|r.error.is_none()).unwrap_or(false);status.set_text(if ok{"Guidance Runtime: Running"}else{"Guidance Runtime: Unavailable"}); if let Ok(c)=UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG)){channel.set_text(format!("Update channel: {}",c.channel));} }
        else if item.id==start_id { let _=runtime.block_on(guidance_ipc::request(Request::new("workload.start_all",json!({})))); }
        else if item.id==stop_id { let _=runtime.block_on(guidance_ipc::request(Request::new("workload.stop_all",json!({})))); }
        else if item.id==check_id { check.set_text("Checking for updates..."); let outcome=UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG)).and_then(|cfg|runtime.block_on(async{let current=Version::parse(env!("CARGO_PKG_VERSION"))?;Ok((cfg.channel,guidance_update::check(&cfg,&current).await?))})); match outcome{Ok((c,Some(u)))=>check.set_text(format!("Update available: {} ({})",u.version,c)),Ok((c,None))=>check.set_text(format!("Up to date ({c})")),Err(_)=>check.set_text("Update check failed")}; }
    }} )?; Ok(())
}
