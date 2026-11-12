use anyhow::{bail, Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::{Path, PathBuf}};
#[cfg(windows)] use std::{io::Cursor, process::Command};

pub const DEFAULT_CONFIG: &str = r"C:\ProgramData\Guidance Runtime\config\updates.toml";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel { Stable, Beta }
impl std::fmt::Display for Channel { fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { write!(f,"{}",match self{Self::Stable=>"stable",Self::Beta=>"beta"}) } }
impl std::str::FromStr for Channel { type Err=anyhow::Error; fn from_str(s:&str)->Result<Self>{match s.to_ascii_lowercase().as_str(){"stable"=>Ok(Self::Stable),"beta"=>Ok(Self::Beta),_=>bail!("channel must be stable or beta")}} }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub repository: String,
    #[serde(default="stable")] pub channel: Channel,
    #[serde(default="install_dir")] pub install_dir: PathBuf,
    #[serde(default="service_name")] pub service_name: String,
    #[serde(default="asset_pattern")] pub asset_pattern: String,
}
fn stable()->Channel{Channel::Stable}
fn install_dir()->PathBuf{PathBuf::from(r"C:\Program Files\Guidance Runtime")}
fn service_name()->String{"GuidanceRuntime".into()}
fn asset_pattern()->String{"windows-x64.zip".into()}
impl UpdateConfig {
    pub fn load(path:&Path)->Result<Self>{let text=fs::read_to_string(path).with_context(||format!("read {}",path.display()))?;Ok(toml::from_str(&text)?) }
    pub fn save(&self,path:&Path)->Result<()>{if let Some(p)=path.parent(){fs::create_dir_all(p)?}fs::write(path,toml::to_string_pretty(self)?)?;Ok(())}
}

#[derive(Debug, Clone, Deserialize)] struct Release { tag_name:String, prerelease:bool, draft:bool, assets:Vec<Asset> }
#[derive(Debug, Clone, Deserialize)] struct Asset { name:String, browser_download_url:String }
#[derive(Debug, Clone, Serialize)] pub struct AvailableUpdate { pub current:Version, pub version:Version, pub tag:String, pub channel:Channel, pub asset_name:String, pub download_url:String, pub checksum_url:Option<String> }

pub async fn check(config:&UpdateConfig,current:&Version)->Result<Option<AvailableUpdate>>{
    let url=format!("https://api.github.com/repos/{}/releases?per_page=100",config.repository);
    let releases=reqwest::Client::new().get(url).header("User-Agent","guidance-runtime-updater").send().await?.error_for_status()?.json::<Vec<Release>>().await?;
    let mut candidates=Vec::new();
    for release in releases.into_iter().filter(|r|!r.draft){
        let raw=release.tag_name.trim_start_matches('v'); let Ok(version)=Version::parse(raw) else{continue};
        let allowed=match config.channel { Channel::Stable=>!release.prerelease&&version.pre.is_empty(), Channel::Beta=>!version.pre.is_empty()&&version.pre.as_str().starts_with("beta.") || !release.prerelease&&version.pre.is_empty() };
        if !allowed || version<=*current {continue}
        if let Some(asset)=release.assets.iter().find(|a|a.name.contains(&config.asset_pattern)&&a.name.ends_with(".zip")){
            let checksum=release.assets.iter().find(|a|a.name.to_ascii_lowercase().contains("sha256")).map(|a|a.browser_download_url.clone());
            candidates.push(AvailableUpdate{current:current.clone(),version,tag:release.tag_name,channel:config.channel,asset_name:asset.name.clone(),download_url:asset.browser_download_url.clone(),checksum_url:checksum});
        }
    }
    candidates.sort_by(|a,b|b.version.cmp(&a.version)); Ok(candidates.into_iter().next())
}

pub async fn download_and_verify(update:&AvailableUpdate)->Result<Vec<u8>>{
    let client=reqwest::Client::new(); let bytes=client.get(&update.download_url).header("User-Agent","guidance-runtime-updater").send().await?.error_for_status()?.bytes().await?.to_vec();
    if let Some(url)=&update.checksum_url { let text=client.get(url).header("User-Agent","guidance-runtime-updater").send().await?.error_for_status()?.text().await?; let expected=text.lines().find(|line|line.contains(&update.asset_name)).or_else(||text.lines().next()).and_then(|line|line.split_whitespace().next()).context("empty checksum file")?.to_ascii_lowercase(); let actual=format!("{:x}",Sha256::digest(&bytes)); if actual!=expected{bail!("SHA-256 mismatch: expected {expected}, got {actual}")}}
    else { bail!("release has no SHA-256 asset; refusing update") }
    Ok(bytes)
}

pub fn install_archive(config:&UpdateConfig,bytes:&[u8])->Result<()> {
    #[cfg(not(windows))] { let _=(config,bytes); bail!("installation is supported on Windows only") }
    #[cfg(windows)] {
        let staging=tempfile::tempdir()?; let mut archive=zip::ZipArchive::new(Cursor::new(bytes))?;
        for i in 0..archive.len(){let mut file=archive.by_index(i)?;let Some(rel)=file.enclosed_name().map(Path::to_path_buf) else{bail!("unsafe archive path")};let out=staging.path().join(rel);if file.is_dir(){fs::create_dir_all(&out)?}else{if let Some(p)=out.parent(){fs::create_dir_all(p)?}let mut target=fs::File::create(&out)?;std::io::copy(&mut file,&mut target)?}}
        let source=find_bin_dir(staging.path())?; let backup=config.install_dir.with_extension("backup");
        run_sc("stop",&config.service_name)?; std::thread::sleep(std::time::Duration::from_secs(2));
        let operation=(||->Result<()>{if backup.exists(){fs::remove_dir_all(&backup)?}if config.install_dir.exists(){copy_dir(&config.install_dir,&backup)?}fs::create_dir_all(&config.install_dir)?;for name in ["guidance-service.exe","guidance.exe","guidance-tray.exe","guidance-updater.exe"]{let src=source.join(name);if src.exists(){fs::copy(src,config.install_dir.join(name))?;}}Ok(())})();
        if let Err(e)=operation{if backup.exists(){let _=copy_dir(&backup,&config.install_dir);}let _=run_sc("start",&config.service_name);return Err(e)}
        if let Err(e)=run_sc("start",&config.service_name){let _=copy_dir(&backup,&config.install_dir);let _=run_sc("start",&config.service_name);return Err(e)}
        Ok(())
    }
}
#[cfg(windows)] fn run_sc(action:&str,service:&str)->Result<()>{let s=Command::new("sc.exe").args([action,service]).status()?;if !s.success(){bail!("sc.exe {action} failed: {s}")}Ok(())}
#[cfg(windows)] fn find_bin_dir(root:&Path)->Result<PathBuf>{for p in walk(root)?{if p.join("guidance-service.exe").exists(){return Ok(p)}}bail!("archive does not contain guidance-service.exe")}
#[cfg(windows)] fn walk(root:&Path)->Result<Vec<PathBuf>>{let mut out=vec![root.to_path_buf()];for e in fs::read_dir(root)?{let p=e?.path();if p.is_dir(){out.extend(walk(&p)?);}}Ok(out)}
#[cfg(windows)] fn copy_dir(src:&Path,dst:&Path)->Result<()>{fs::create_dir_all(dst)?;for e in fs::read_dir(src)?{let e=e?;let to=dst.join(e.file_name());if e.path().is_dir(){copy_dir(&e.path(),&to)?}else{fs::copy(e.path(),to)?;}}Ok(())}

#[cfg(test)] mod tests { use super::*; #[test] fn channel_parse(){assert_eq!("stable".parse::<Channel>().unwrap(),Channel::Stable);assert!("nightly".parse::<Channel>().is_err());} }
