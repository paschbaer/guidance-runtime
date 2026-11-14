// GUI app: no console window in release builds; debug keeps one for dev output.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

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
        .with_icon(app_icon())
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

/// Renders the tray icon in code so the repository carries no binary asset.
/// Style taken from the operator's "i" icon (pixel analysis of icon.png):
/// a blue badge with a vertical gradient (bright blue ~RGB 3,80,166 at the
/// top fading into dark navy ~RGB 31,46,97 at the bottom) and a white glyph
/// — here a "G" (ring open to the right with crossbar), supersampled 2x2.
#[cfg(windows)]
fn app_icon() -> tray_icon::Icon {
    const SIZE: u32 = 32;
    const BLUE_BRIGHT: [f64; 3] = [3.0, 80.0, 166.0];
    const BLUE_DARK: [f64; 3] = [31.0, 46.0, 97.0];
    let cx = 15.5;
    let cy = 15.5;
    let (r_out, r_in) = (10.0, 6.2);

    // Rounded-square badge mask (corner radius 7) covering the full canvas.
    let in_badge = |x: u32, y: u32| -> bool {
        let fx = x as f64;
        let fy = y as f64;
        let dx = if fx < 7.0 {
            7.0 - fx
        } else if fx > 24.0 {
            fx - 24.0
        } else {
            0.0
        };
        let dy = if fy < 7.0 {
            7.0 - fy
        } else if fy > 24.0 {
            fy - 24.0
        } else {
            0.0
        };
        dx * dx + dy * dy <= 49.0
    };
    let is_bar = |x: f64, y: f64| (14.0..18.0).contains(&y) && (14.0..=23.0).contains(&x);
    let is_stub = |x: f64, y: f64| (16.0..=23.0).contains(&y) && (19.0..=24.0).contains(&x);
    let is_ring = |x: f64, y: f64| {
        let r = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        let deg = (y - cy).atan2(x - cx).to_degrees();
        (r_in..=r_out).contains(&r) && !(-40.0..45.0).contains(&deg)
    };

    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            // 2x2 supersampling for badge edges and the glyph.
            let mut badge_cov = 0u32;
            let mut glyph_cov = 0u32;
            for sy in [0.25, 0.75] {
                for sx in [0.25, 0.75] {
                    let fx = x as f64 + sx;
                    let fy = y as f64 + sy;
                    if in_badge(x, y) {
                        badge_cov += 1;
                    }
                    if is_bar(fx, fy) || is_stub(fx, fy) || is_ring(fx, fy) {
                        glyph_cov += 1;
                    }
                }
            }
            if badge_cov == 0 && glyph_cov == 0 {
                continue;
            }
            // Vertical gradient, bright blue at the top into dark navy below.
            let t = y as f64 / (SIZE - 1) as f64;
            let badge = [
                BLUE_BRIGHT[0] + (BLUE_DARK[0] - BLUE_BRIGHT[0]) * t,
                BLUE_BRIGHT[1] + (BLUE_DARK[1] - BLUE_BRIGHT[1]) * t,
                BLUE_BRIGHT[2] + (BLUE_DARK[2] - BLUE_BRIGHT[2]) * t,
            ];
            let i = (y as usize * SIZE as usize + x as usize) * 4;
            // White glyph (coverage ga) composited over the badge (alpha ba):
            // out = white*ga + badge*ba*(1-ga); alpha = ga + ba*(1-ga).
            let ga = glyph_cov as f64 / 4.0;
            let ba = badge_cov as f64 / 4.0;
            let out_a = ga + ba * (1.0 - ga);
            if out_a <= 0.0 {
                continue;
            }
            rgba[i] = ((255.0 * ga + badge[0] * ba * (1.0 - ga)) / out_a) as u8;
            rgba[i + 1] = ((255.0 * ga + badge[1] * ba * (1.0 - ga)) / out_a) as u8;
            rgba[i + 2] = ((255.0 * ga + badge[2] * ba * (1.0 - ga)) / out_a) as u8;
            rgba[i + 3] = (out_a * 255.0) as u8;
        }
    }
    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE).expect("static RGBA icon data")
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
