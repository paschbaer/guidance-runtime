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

/// How `update check` renders its result. The default keeps the historical
/// human-readable output, so an operator gets the pre-feature behaviour unless
/// they explicitly ask for JSON (SC-001).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
enum Format {
    #[default]
    Text,
    Json,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
#[derive(Subcommand, Debug)]
enum UpdateCommand {
    Check {
        #[arg(long,default_value=guidance_update::DEFAULT_CONFIG)]
        config: PathBuf,
        /// Check only this target. Omitted means every configured target.
        #[arg(long)]
        target: Option<String>,
        /// Output format: human-readable text (default) or one JSON document.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
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
    // Usage errors exit 1 per update-cli.md §2; only help/version (which write
    // to stdout) exit 0. Exit 2 stays reserved for "at least one target failed".
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().expect("failed to print clap error");
            std::process::exit(if err.use_stderr() { 1 } else { 0 });
        }
    };
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
    println!("{}", serde_json::to_string_pretty(&response.result)?);
    Ok(())
}
async fn update(command: UpdateCommand) -> Result<()> {
    // Update runs live in this CLI process, not in the service, so their
    // outcomes would otherwise exist only on the console. Log them to the
    // shared logs directory (daily rotation, JSON) next to service.log.
    #[cfg(windows)]
    init_update_logging();
    tracing::info!(command = ?command, "update command invoked");
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
                tracing::info!(
                    target = ?target,
                    channel = ?channel,
                    "update channel changed"
                );
            }
            println!("{message}")
        }
        UpdateCommand::Check {
            config,
            target,
            format,
        } => {
            let loaded = UpdateConfig::load(&config)?;
            // `--target` narrows the run itself, not just the printing: a
            // check must not read the sources (or consult the credential
            // stores) of targets the operator explicitly excluded.
            let cfg = match target.as_deref() {
                Some(name) => loaded.select_target(name)?,
                None => loaded,
            };
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
                // JSON changes the rendering only, never this exit status.
                let outcome =
                    guidance_update::check_update_with(&source, only, None, &current).await?;
                tracing::info!(
                    target = %only.name,
                    outcome = %outcome.message(),
                    "update check finished"
                );
                match format {
                    Format::Text => println!("{}", outcome.message()),
                    Format::Json => {
                        let run = guidance_update::UpdateRun::new(vec![
                            guidance_update::check_from_outcome(only, &outcome, &current),
                        ]);
                        println!("{}", guidance_update::render_json_now(&run)?);
                    }
                }
                return Ok(());
            }

            let run = guidance_update::check_all(&source, &store, &cfg, &current).await;
            for t in &run.targets {
                tracing::info!(
                    target = %t.target,
                    state = %t.state,
                    installed = ?t.installed_version,
                    offered = ?t.offered_version,
                    channel = %t.channel,
                    "update check"
                );
            }
            tracing::info!(exit_status = run.exit_status(), "update check finished");
            match format {
                Format::Text => print!("{}", guidance_update::render_run(&run)),
                Format::Json => println!("{}", guidance_update::render_json_now(&run)?),
            }
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
                tracing::info!(
                    target = ?target,
                    current = %current,
                    updater = %temp.display(),
                    "update install requested; detached updater started"
                );
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

/// Daily-rolling JSON log for update commands, next to service.log. Only the
/// outcome events go here; the rendered report stays on the console.
#[cfg(windows)]
fn init_update_logging() {
    const UPDATE_LOG_DIR: &str = "C:/ProgramData/Guidance Runtime/logs";
    let dir = PathBuf::from(UPDATE_LOG_DIR);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let appender = tracing_appender::rolling::daily(&dir, "update.log");
    tracing_subscriber::fmt()
        .json()
        .with_writer(appender)
        .init();
}
