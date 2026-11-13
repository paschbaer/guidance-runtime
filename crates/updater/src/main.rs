use anyhow::Result;
use clap::{Parser, Subcommand};
use guidance_update::{Channel, UpdateConfig};
use semver::Version;
use std::{path::PathBuf, time::Duration};

/// Output format for `check`; text is the historical default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
enum Format {
    #[default]
    Text,
    Json,
}
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
        /// Output format: human-readable text (default) or one JSON document.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    Install {
        #[arg(long)]
        current: Option<Version>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        wait_pid: Option<u32>,
        #[arg(long)]
        target: Option<String>,
    },
    Channel {
        channel: Option<Channel>,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    // Usage errors exit 1 per update-cli.md §2; only help/version (which write
    // to stdout) exit 0. Exit 2 stays reserved for "at least one target failed".
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().expect("failed to print clap error");
            std::process::exit(if err.use_stderr() { 1 } else { 0 });
        }
    };
    let cfg = UpdateConfig::load(&cli.config)?;
    match cli.command {
        Cmd::Channel { channel } => {
            let mut cfg = cfg;
            let message = guidance_update::set_or_get_channel(&mut cfg, None, channel)?;
            if channel.is_some() {
                cfg.save(&cli.config)?;
            }
            println!("{message}")
        }
        Cmd::Check { current, format } => {
            let v = match current {
                Some(v) => v,
                None => guidance_update::current_version()?,
            };
            let source = guidance_update::GitHubReleases::new();
            let store = guidance_update::PlatformCredentialStore::new();

            if guidance_update::uses_legacy_output(&cfg, None) {
                let only = cfg.single_target()?;
                let outcome = guidance_update::check_update_with(&source, only, None, &v).await?;
                match format {
                    Format::Text => println!("{}", outcome.message()),
                    Format::Json => {
                        let run = guidance_update::UpdateRun::new(vec![
                            guidance_update::check_from_outcome(only, &outcome, &v),
                        ]);
                        println!("{}", guidance_update::render_json_now(&run)?);
                    }
                }
                return Ok(());
            }

            let run = guidance_update::check_all(&source, &store, &cfg, &v).await;
            match format {
                Format::Text => print!("{}", guidance_update::render_run(&run)),
                Format::Json => println!("{}", guidance_update::render_json_now(&run)?),
            }
            std::process::exit(run.exit_status() as i32);
        }
        Cmd::Install {
            current,
            yes,
            wait_pid,
            target,
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
            // `--target` narrows the run itself: other targets' sources and
            // credential stores are not contacted. Omission selects every
            // configured target; only those reporting an available update are
            // installed.
            let cfg = match target {
                Some(name) => cfg.select_target(&name)?,
                None => cfg,
            };
            let source = guidance_update::GitHubReleases::new();
            let store = guidance_update::PlatformCredentialStore::new();
            let run = guidance_update::install_all_default(&source, &store, &cfg, &v).await;
            print!("{}", run.render());
            std::process::exit(run.exit_status() as i32);
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
