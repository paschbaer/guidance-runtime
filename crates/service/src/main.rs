use anyhow::Result;
use clap::Parser;
use guidance_runtime::Runtime;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
struct A {
    #[arg(
        long,
        default_value = "C:/ProgramData/Guidance Runtime/config/runtime.toml"
    )]
    config: PathBuf,
    /// Run in the foreground and wait for Ctrl+C instead of the Windows
    /// service dispatcher (development).
    #[arg(long)]
    console: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let a = A::parse();

    #[cfg(windows)]
    {
        if !a.console {
            // As a service, stdout goes nowhere: send the JSON logs to the
            // same logs directory the runtime.toml configures for workloads.
            let log_dir = PathBuf::from("C:/ProgramData/Guidance Runtime/logs");
            let _ = std::fs::create_dir_all(&log_dir);
            let appender = tracing_appender::rolling::daily(&log_dir, "service.log");
            tracing_subscriber::fmt()
                .json()
                .with_writer(appender)
                .init();
            // Blocks until the SCM stops the service; the dispatcher thread
            // runs its own tokio runtime (see service_mode::service_main).
            return service_mode::start();
        }
    }

    tracing_subscriber::fmt().json().init();
    run_console(a.config).await
}

/// Foreground mode: load the runtime, start enabled workloads, serve IPC, and
/// stop everything on Ctrl+C.
async fn run_console(config: PathBuf) -> Result<()> {
    let r = Runtime::load(&config).await?;
    r.start_enabled().await;
    let h = tokio::spawn(guidance_ipc::serve(r.clone()));
    info!("running in console mode; press Ctrl+C to stop");
    tokio::signal::ctrl_c().await?;
    r.stop_all().await;
    h.abort();
    Ok(())
}

#[cfg(windows)]
mod service_mode {
    use super::*;
    use std::ffi::OsString;
    use std::time::Duration;
    use tokio::sync::oneshot;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{
        register, ServiceControlHandlerResult, ServiceStatusHandle,
    };
    use windows_service::{define_windows_service, service_dispatcher};

    /// Must match the WiX ServiceInstall Name and install-dev.ps1.
    const SERVICE_NAME: &str = "GuidanceRuntime";
    const DEFAULT_CONFIG: &str = "C:/ProgramData/Guidance Runtime/config/runtime.toml";

    define_windows_service!(ffi_service_main, service_main);

    /// Enters the SCM dispatcher. Only returns when the service has stopped.
    pub fn start() -> Result<()> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
            .map_err(|e| anyhow::anyhow!("service dispatcher failed: {e}"))
    }

    fn service_main(args: Vec<OsString>) {
        // The dispatcher thread has no async runtime; build one for the body.
        let result = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt.block_on(run(config_from(&args))),
            Err(e) => Err(anyhow::anyhow!("cannot build tokio runtime: {e}")),
        };
        if let Err(e) = result {
            tracing::error!("service terminated with error: {e:#}");
        }
    }

    /// argv[0] is the service name, so look for `--config <path>` afterwards.
    fn config_from(args: &[OsString]) -> PathBuf {
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if a == "--config" {
                if let Some(v) = it.next() {
                    return PathBuf::from(v);
                }
            }
        }
        PathBuf::from(DEFAULT_CONFIG)
    }

    async fn run(config: PathBuf) -> Result<()> {
        let (stop_tx, stop_rx) = oneshot::channel();
        // The SCM callback is FnMut: stash the sender so STOP can fire it once.
        let stop_tx = std::sync::Mutex::new(Some(stop_tx));
        let status_handle = register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Some(tx) = stop_tx.lock().expect("stop lock").take() {
                    let _ = tx.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
        set_state(&status_handle, ServiceState::StartPending)?;

        // Init must complete before RUNNING is reported, or the SCM kills the
        // process after its wait hint once it thinks the service stalled.
        let r = Runtime::load(&config).await?;
        r.start_enabled().await;
        let h = tokio::spawn(guidance_ipc::serve(r.clone()));

        set_state(&status_handle, ServiceState::Running)?;
        info!("service running");
        tokio::select! {
            _ = stop_rx => { info!("stop control received"); }
            _ = tokio::signal::ctrl_c() => { info!("ctrl-c received"); }
        }

        set_state(&status_handle, ServiceState::StopPending)?;
        r.stop_all().await;
        h.abort();
        set_state(&status_handle, ServiceState::Stopped)?;
        Ok(())
    }

    fn set_state(handle: &ServiceStatusHandle, state: ServiceState) -> windows_service::Result<()> {
        handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: match state {
                ServiceState::StopPending => ServiceControlAccept::empty(),
                _ => ServiceControlAccept::STOP,
            },
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(30),
            process_id: None,
        })
    }
}
