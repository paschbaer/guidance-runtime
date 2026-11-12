#[cfg(windows)]
use anyhow::Context;
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use guidance_protocol::Request;
use guidance_update::{Channel, UpdateConfig};
use semver::Version;
use serde_json::json;
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command as OsCommand;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true)]
    json: bool,
}
#[derive(Subcommand)]
enum Command {
    Status,
    List,
    Start {
        id: String,
    },
    Stop {
        id: String,
    },
    Restart {
        id: String,
    },
    Logs {
        id: String,
        #[arg(long, default_value_t = 100)]
        tail: usize,
    },
    StartAll,
    StopAll,
    Doctor,
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
}
#[derive(Subcommand)]
enum UpdateCommand {
    Check {
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
    },
    Install {
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
        #[arg(long)]
        yes: bool,
    },
    Channel {
        channel: Option<Channel>,
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let json_output = cli.json;
    let command = match cli.command {
        Command::Update { command } => return update(command).await,
        other => other,
    };
    let (method, params) = match command {
        Command::Status => ("runtime.status", json!({})),
        Command::List => ("workload.list", json!({})),
        Command::Start { id } => ("workload.start", json!({"workload_id":id})),
        Command::Stop { id } => ("workload.stop", json!({"workload_id":id})),
        Command::Restart { id } => ("workload.restart", json!({"workload_id":id})),
        Command::Logs { id, tail } => ("workload.logs", json!({"workload_id":id,"tail":tail})),
        Command::StartAll => ("workload.start_all", json!({})),
        Command::StopAll => ("workload.stop_all", json!({})),
        Command::Doctor => ("diagnostics.summary", json!({})),
        Command::Update { .. } => unreachable!(),
    };
    let response = guidance_ipc::request(Request::new(method, params)).await?;
    if let Some(error) = response.error {
        bail!("{}: {}", error.code, error.message)
    }
    println!(
        "{}",
        if json_output {
            serde_json::to_string(&response.result)?
        } else {
            serde_json::to_string_pretty(&response.result)?
        }
    );
    Ok(())
}
async fn update(command: UpdateCommand) -> Result<()> {
    match command {
        UpdateCommand::Channel { channel, config } => {
            let mut cfg = UpdateConfig::load(&config)?;
            if let Some(c) = channel {
                cfg.channel = c;
                cfg.save(&config)?;
                println!("Update channel set to {c}")
            } else {
                println!("{}", cfg.channel)
            }
        }
        UpdateCommand::Check { config } => {
            let cfg = UpdateConfig::load(&config)?;
            let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
            match guidance_update::check(&cfg, &current).await? {
                Some(u) => println!(
                    "Update available: {} -> {} ({})",
                    u.current, u.version, u.channel
                ),
                None => println!("No update available on {} channel", cfg.channel),
            }
        }
        UpdateCommand::Install { config, yes } => {
            if !yes {
                bail!("installation requires --yes")
            }
            #[cfg(not(windows))]
            bail!("installation is supported on Windows only");
            #[cfg(windows)]
            {
                let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
                let source = std::env::current_exe()?.with_file_name("guidance-updater.exe");
                let temp = std::env::temp_dir()
                    .join(format!("guidance-updater-{}.exe", std::process::id()));
                std::fs::copy(&source, &temp)
                    .with_context(|| format!("copy {}", source.display()))?;
                OsCommand::new(&temp)
                    .args([
                        "--config",
                        &config.display().to_string(),
                        "install",
                        "--current",
                        &current.to_string(),
                        "--yes",
                        "--wait-pid",
                        &std::process::id().to_string(),
                    ])
                    .spawn()?;
                println!("Updater started. It will apply the update after this CLI process exits.");
            }
        }
    }
    Ok(())
}
