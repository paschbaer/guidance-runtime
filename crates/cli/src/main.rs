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
        /// Check only this target. Omitted means every configured target.
        #[arg(long)]
        target: Option<String>,
    },
    Install {
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
        /// Install only this target. Omitted means every target with an update.
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    Channel {
        channel: Option<Channel>,
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
        /// Required when more than one target is configured.
        #[arg(long)]
        target: Option<String>,
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
        UpdateCommand::Channel {
            channel,
            config,
            target,
        } => {
            let mut cfg = UpdateConfig::load(&config)?;
            let message =
                guidance_update::set_or_get_channel(&mut cfg, target.as_deref(), channel)?;
            if channel.is_some() {
                cfg.save(&config)?;
            }
            println!("{message}")
        }
        UpdateCommand::Check { config, target } => {
            let cfg = UpdateConfig::load(&config)?;
            let current = guidance_update::current_version()?;
            let source = guidance_update::GitHubReleases::new();
            let store = guidance_update::PlatformCredentialStore::new();

            // A file in the historical flat form keeps the historical output
            // contract: same wording, same single-target shape (SC-001). Only
            // an explicit [[targets]] file gets per-target lines.
            if guidance_update::uses_legacy_output(&cfg, target.as_deref()) {
                let only = cfg.single_target()?;
                // `?` here is deliberate: the historical contract reported a
                // source failure as an error with exit status 1, and an
                // operator's existing script may depend on that distinction.
                let outcome =
                    guidance_update::check_update_with(&source, only, None, &current).await?;
                println!("{}", outcome.message());
                return Ok(());
            }

            let run = guidance_update::check_all(&source, &store, &cfg, &current).await;
            if let Some(name) = target {
                match run.targets.iter().find(|t| t.target == name) {
                    Some(check) => println!("{}", guidance_update::render_check(check)),
                    None => bail!("target '{name}' is not configured"),
                }
                // Report the status of the target that was asked for, not of
                // targets this run was told to ignore.
                let status = run
                    .targets
                    .iter()
                    .find(|t| t.target == name)
                    .map(|t| t.state.is_failure())
                    .unwrap_or(false);
                std::process::exit(if status { 2 } else { 0 });
            }
            print!("{}", guidance_update::render_run(&run));
            std::process::exit(run.exit_status() as i32);
        }
        UpdateCommand::Install {
            config,
            target,
            yes,
        } => {
            if !yes {
                bail!("installation requires --yes")
            }
            #[cfg(not(windows))]
            {
                let _ = (config, target);
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
                let mut args = vec![
                    "--config".to_string(),
                    config.display().to_string(),
                    "install".to_string(),
                    "--current".to_string(),
                    current.to_string(),
                    "--yes".to_string(),
                    "--wait-pid".to_string(),
                    std::process::id().to_string(),
                ];
                // The target selection must reach the updater; dropping it here
                // would make the flag silently select the wrong product.
                if let Some(name) = target {
                    args.push("--target".to_string());
                    args.push(name);
                }
                OsCommand::new(&temp).args(&args).spawn()?;
                println!("Updater started. It will apply the update after this CLI process exits.");
            }
        }
    }
    Ok(())
}
