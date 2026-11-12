use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use guidance_update::{Channel, UpdateConfig};
use semver::Version;
use std::{path::PathBuf, time::Duration};
#[derive(Parser)]
#[command(version, about = "Guidance Runtime update checker and installer")]
struct Cli {
    #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
    config: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    Check {
        #[arg(long)]
        current: Option<Version>,
    },
    Install {
        #[arg(long)]
        current: Option<Version>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        wait_pid: Option<u32>,
    },
    Channel {
        channel: Option<Channel>,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = UpdateConfig::load(&cli.config)?;
    match cli.command {
        Cmd::Channel { channel } => {
            println!(
                "{}",
                guidance_update::set_or_get_channel(&cli.config, channel)?
            )
        }
        Cmd::Check { current } => {
            let v = match current {
                Some(v) => v,
                None => guidance_update::current_version()?,
            };
            let outcome = guidance_update::check_update(&cfg, &v).await?;
            println!("{}", outcome.message())
        }
        Cmd::Install {
            current,
            yes,
            wait_pid,
        } => {
            if !yes {
                anyhow::bail!("installation requires --yes")
            };
            if let Some(pid) = wait_pid {
                wait_for_exit(pid, Duration::from_secs(60))?
            }
            let v = match current {
                Some(v) => v,
                None => guidance_update::current_version()?,
            };
            let u = guidance_update::check(&cfg, &v)
                .await?
                .context("no update available")?;
            println!("Downloading {}", u.asset_name);
            let b = guidance_update::download_and_verify(&u).await?;
            guidance_update::install_archive(&cfg, &b)?;
            println!("Installed Guidance Runtime {}", u.version)
        }
    }
    Ok(())
}
fn wait_for_exit(pid: u32, timeout: Duration) -> Result<()> {
    #[cfg(not(windows))]
    {
        let _ = (pid, timeout);
        Ok(())
    }
    #[cfg(windows)]
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            let out = std::process::Command::new("tasklist.exe")
                .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
                .output()?;
            let text = String::from_utf8_lossy(&out.stdout);
            if !text.contains(&pid.to_string()) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        anyhow::bail!("timed out waiting for process {pid}")
    }
}
