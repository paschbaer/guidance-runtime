#[cfg(windows)]
use anyhow::Context;
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use guidance_protocol::{Method, Request};
use guidance_update::{Channel, UpdateConfig};
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
        Command::Status => (Method::RuntimeStatus, json!({})),
        Command::List => (Method::WorkloadList, json!({})),
        Command::Start { id } => (Method::WorkloadStart, json!({ "workload_id": id })),
        Command::Stop { id } => (Method::WorkloadStop, json!({ "workload_id": id })),
        Command::Restart { id } => (Method::WorkloadRestart, json!({ "workload_id": id })),
        Command::Logs { id, tail } => (
            Method::WorkloadLogs,
            json!({ "workload_id": id, "tail": tail }),
        ),
        Command::StartAll => (Method::WorkloadStartAll, json!({})),
        Command::StopAll => (Method::WorkloadStopAll, json!({})),
        Command::Doctor => (Method::DiagnosticsSummary, json!({})),
        // Dispatched above via the early return for `Command::Update`.
        Command::Update { .. } => unreachable!("update commands are handled before IPC dispatch"),
    };
    let response = guidance_ipc::request(Request::new(method.as_str(), params)).await?;
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
            println!("{}", guidance_update::set_or_get_channel(&config, channel)?)
        }
        UpdateCommand::Check { config } => {
            let cfg = UpdateConfig::load(&config)?;
            let current = guidance_update::current_version()?;
            let outcome = guidance_update::check_update(&cfg, &current).await?;
            println!("{}", outcome.message())
        }
        UpdateCommand::Install { config, yes } => {
            if !yes {
                bail!("installation requires --yes")
            }
            #[cfg(not(windows))]
            {
                let _ = config;
                bail!("installation is supported on Windows only");
            }
            #[cfg(windows)]
            {
                let current = guidance_update::current_version()?;
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
