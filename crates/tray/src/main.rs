#[cfg(not(windows))]
fn main() {
    eprintln!("guidance-tray is available on Windows only");
}
#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use guidance_update::UpdateConfig;
    use std::path::Path;
    use tray_icon::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        TrayIconBuilder,
    };
    use winit::event_loop::{ControlFlow, EventLoop};

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let menu = Menu::new();
    let status = MenuItem::new("Guidance Runtime: connecting", false, None);
    let channel_text = UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG))
        .map(|c| format!("Update channel: {}", channel_summary(&c)))
        .unwrap_or_else(|_| "Update channel: not configured".into());
    let channel = MenuItem::new(channel_text, false, None);
    let check = MenuItem::new("Check for updates", true, None);
    let refresh = MenuItem::new("Refresh", true, None);
    let start = MenuItem::new("Start All Enabled", true, None);
    let stop = MenuItem::new("Stop All", true, None);
    let exit = MenuItem::new("Exit Tray", true, None);
    menu.append_items(&[
        &status,
        &channel,
        &check,
        &PredefinedMenuItem::separator(),
        &start,
        &stop,
        &refresh,
        &PredefinedMenuItem::separator(),
        &exit,
    ])?;
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Guidance Runtime")
        .build()?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (refresh_id, start_id, stop_id, check_id, exit_id) = (
        refresh.id().clone(),
        start.id().clone(),
        stop.id().clone(),
        check.id().clone(),
        exit.id().clone(),
    );
    event_loop.run_app(&mut TrayApp {
        runtime,
        refresh_id,
        start_id,
        stop_id,
        check_id,
        exit_id,
        status,
        channel,
        check,
    })?;
    Ok(())
}

/// A short channel summary for the tray label: the single channel when one
/// target is configured, otherwise a count, so the label cannot imply that one
/// channel governs every target.
#[cfg(windows)]
fn channel_summary(config: &guidance_update::UpdateConfig) -> String {
    let targets = config.targets();
    if let [only] = targets {
        only.channel.to_string()
    } else {
        format!("{} targets", targets.len())
    }
}

#[cfg(windows)]
struct TrayApp {
    runtime: tokio::runtime::Runtime,
    refresh_id: tray_icon::menu::MenuId,
    start_id: tray_icon::menu::MenuId,
    stop_id: tray_icon::menu::MenuId,
    check_id: tray_icon::menu::MenuId,
    exit_id: tray_icon::menu::MenuId,
    status: tray_icon::menu::MenuItem,
    channel: tray_icon::menu::MenuItem,
    check: tray_icon::menu::MenuItem,
}

#[cfg(windows)]
impl winit::application::ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, elwt: &winit::event_loop::ActiveEventLoop) {
        use guidance_protocol::{Method, Request};
        use guidance_update::UpdateConfig;
        use semver::Version;
        use serde_json::json;
        use std::path::Path;
        use tray_icon::menu::MenuEvent;

        let Ok(item) = MenuEvent::receiver().try_recv() else {
            return;
        };
        if item.id == self.exit_id {
            elwt.exit();
        } else if item.id == self.refresh_id {
            let ok = self
                .runtime
                .block_on(guidance_ipc::request(Request::new(
                    Method::RuntimeStatus.as_str(),
                    json!({}),
                )))
                .map(|r| r.error.is_none())
                .unwrap_or(false);
            self.status.set_text(if ok {
                "Guidance Runtime: Running"
            } else {
                "Guidance Runtime: Unavailable"
            });
            if let Ok(c) = UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG)) {
                self.channel
                    .set_text(format!("Update channel: {}", channel_summary(&c)));
            }
        } else if item.id == self.start_id {
            let _ = self.runtime.block_on(guidance_ipc::request(Request::new(
                Method::WorkloadStartAll.as_str(),
                json!({}),
            )));
        } else if item.id == self.stop_id {
            let _ = self.runtime.block_on(guidance_ipc::request(Request::new(
                Method::WorkloadStopAll.as_str(),
                json!({}),
            )));
        } else if item.id == self.check_id {
            self.check.set_text("Checking for updates...");
            let outcome =
                UpdateConfig::load(Path::new(guidance_update::DEFAULT_CONFIG)).and_then(|cfg| {
                    self.runtime.block_on(async {
                        let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
                        let source = guidance_update::GitHubReleases::new();
                        let store = guidance_update::PlatformCredentialStore::new();
                        Ok(guidance_update::check_all(&source, &store, &cfg, &current).await)
                    })
                });
            match outcome {
                Ok(run) => self.check.set_text(check_summary(&run)),
                Err(_) => self.check.set_text("Update check failed"),
            };
        }
    }
}

/// Summarises a check run for the tray label.
///
/// A failure must never be reported as "Up to date": the fan-out converts
/// source and credential failures into states instead of errors, so a run where
/// every target failed still arrives here as `Ok`. Reporting it as up to date
/// is the exact fail-open this feature exists to remove.
#[cfg(windows)]
fn check_summary(run: &guidance_update::UpdateRun) -> String {
    use guidance_update::TargetState;

    let failed: Vec<&str> = run
        .targets
        .iter()
        .filter(|t| t.state.is_failure())
        .map(|t| t.target.as_str())
        .collect();
    if !failed.is_empty() {
        return format!("Update problem: {}", failed.join(", "));
    }

    let available: Vec<_> = run
        .targets
        .iter()
        .filter(|t| t.state == TargetState::UpdateAvailable)
        .collect();
    if available.is_empty() {
        "Up to date".to_string()
    } else {
        let parts: Vec<String> = available
            .iter()
            .map(|t| {
                format!(
                    "{} {}",
                    t.target,
                    t.offered_version.as_deref().unwrap_or("?")
                )
            })
            .collect();
        format!("Update available: {}", parts.join(", "))
    }
}

#[cfg(all(windows, test))]
mod tests {
    use super::*;

    fn check(name: &str, state: guidance_update::TargetState) -> guidance_update::TargetCheck {
        guidance_update::TargetCheck::new(name, state, Some("1.0.0".into()), None, "stable", None)
    }

    /// The defect this guards: a fan-out that swallows every failure must not
    /// reach the tray as "Up to date".
    #[test]
    fn an_all_failing_run_is_not_reported_as_up_to_date() {
        let run = guidance_update::UpdateRun::new(vec![
            check("a", guidance_update::TargetState::Inaccessible),
            check("b", guidance_update::TargetState::NetworkError),
        ]);
        let summary = check_summary(&run);
        assert!(!summary.contains("Up to date"), "{summary}");
        assert!(summary.contains("Update problem"), "{summary}");
        assert!(summary.contains('a') && summary.contains('b'), "{summary}");
    }

    #[test]
    fn a_clean_run_still_reads_up_to_date() {
        let run = guidance_update::UpdateRun::new(vec![check(
            "a",
            guidance_update::TargetState::UpToDate,
        )]);
        assert_eq!(check_summary(&run), "Up to date");
    }
}
