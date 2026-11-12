use anyhow::{Context, Result};
use guidance_protocol::WorkloadStatus;
use parking_lot::RwLock;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
    time::sleep,
};
#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(default = "logs")]
    pub log_dir: PathBuf,
    #[serde(default)]
    pub workloads: Vec<Workload>,
}
fn logs() -> PathBuf {
    "logs".into()
}
#[derive(Clone, Deserialize)]
pub struct Workload {
    pub id: String,
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub restart: Restart,
}
#[derive(Clone, Deserialize)]
pub struct Restart {
    #[serde(default = "policy")]
    pub policy: String,
    #[serde(default = "attempts")]
    pub max_attempts: u32,
    #[serde(default = "delay")]
    pub delay_seconds: u64,
}
impl Default for Restart {
    fn default() -> Self {
        Self {
            policy: policy(),
            max_attempts: attempts(),
            delay_seconds: delay(),
        }
    }
}
fn policy() -> String {
    "on_failure".into()
}
fn attempts() -> u32 {
    5
}
fn delay() -> u64 {
    10
}
struct State {
    name: String,
    pid: Option<u32>,
    restarts: u32,
    error: Option<String>,
}
struct Entry {
    cfg: Workload,
    state: RwLock<State>,
    child: Mutex<Option<Child>>,
    desired: RwLock<bool>,
}
#[derive(Clone)]
pub struct Runtime {
    entries: Arc<HashMap<String, Arc<Entry>>>,
    log_dir: PathBuf,
}
impl Runtime {
    pub async fn load(path: &Path) -> Result<Self> {
        let raw = tokio::fs::read_to_string(path).await?;
        let c: Config = toml::from_str(&raw)?;
        tokio::fs::create_dir_all(&c.log_dir).await?;
        let mut m = HashMap::new();
        for x in c.workloads {
            if m.contains_key(&x.id) {
                anyhow::bail!("duplicate workload {}", x.id)
            }
            let enabled = x.enabled;
            m.insert(
                x.id.clone(),
                Arc::new(Entry {
                    cfg: x,
                    state: RwLock::new(State {
                        name: "Stopped".into(),
                        pid: None,
                        restarts: 0,
                        error: None,
                    }),
                    child: Mutex::new(None),
                    desired: RwLock::new(enabled),
                }),
            );
        }
        Ok(Self {
            entries: Arc::new(m),
            log_dir: c.log_dir,
        })
    }
    pub fn list(&self) -> Vec<WorkloadStatus> {
        self.entries
            .iter()
            .map(|(id, e)| {
                let s = e.state.read();
                WorkloadStatus {
                    id: id.clone(),
                    state: s.name.clone(),
                    pid: s.pid,
                    restart_count: s.restarts,
                    last_error: s.error.clone(),
                    endpoint: e.cfg.endpoint.clone(),
                }
            })
            .collect()
    }
    fn get(&self, id: &str) -> Result<Arc<Entry>> {
        self.entries
            .get(id)
            .cloned()
            .with_context(|| format!("workload not found: {id}"))
    }
    pub async fn start_enabled(&self) {
        for (id, e) in self.entries.iter() {
            if *e.desired.read() {
                let _ = self.start(id).await;
            }
        }
    }
    pub async fn start(&self, id: &str) -> Result<()> {
        let e = self.get(id)?;
        *e.desired.write() = true;
        if matches!(e.state.read().name.as_str(), "Running" | "Starting") {
            return Ok(());
        }
        e.state.write().name = "Starting".into();
        let path = self.log_dir.join(format!("{id}.log"));
        let out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let err = out.try_clone()?;
        let mut c = Command::new(&e.cfg.command);
        c.args(&e.cfg.args)
            .envs(&e.cfg.env)
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err));
        if let Some(d) = &e.cfg.working_dir {
            c.current_dir(d);
        }
        match c.spawn() {
            Ok(ch) => {
                let pid = ch.id();
                *e.child.lock().await = Some(ch);
                {
                    let mut s = e.state.write();
                    s.name = "Running".into();
                    s.pid = pid;
                    s.error = None;
                }
                self.monitor(e);
                Ok(())
            }
            Err(x) => {
                let mut s = e.state.write();
                s.name = "Failed".into();
                s.error = Some(x.to_string());
                Err(x.into())
            }
        }
    }
    fn monitor(&self, e: Arc<Entry>) {
        let rt = self.clone();
        tokio::spawn(async move {
            let status = {
                let mut g = e.child.lock().await;
                match g.as_mut() {
                    Some(c) => c.wait().await.ok(),
                    None => None,
                }
            };
            *e.child.lock().await = None;
            let ok = status.map(|x| x.success()).unwrap_or(false);
            {
                let mut s = e.state.write();
                s.pid = None;
                s.name = "Stopped".into();
                if !ok {
                    s.error = Some(format!("exit: {status:?}"));
                }
            }
            let retry = *e.desired.read()
                && (e.cfg.restart.policy == "always"
                    || (e.cfg.restart.policy == "on_failure" && !ok));
            if retry {
                let n = {
                    let mut s = e.state.write();
                    s.restarts += 1;
                    s.restarts
                };
                if n <= e.cfg.restart.max_attempts {
                    e.state.write().name = "Restarting".into();
                    sleep(Duration::from_secs(e.cfg.restart.delay_seconds)).await;
                    if *e.desired.read() {
                        let _ = rt.start(&e.cfg.id).await;
                    }
                } else {
                    e.state.write().name = "Failed".into();
                }
            }
        });
    }
    pub async fn stop(&self, id: &str) -> Result<()> {
        let e = self.get(id)?;
        *e.desired.write() = false;
        e.state.write().name = "Stopping".into();
        let mut g = e.child.lock().await;
        if let Some(c) = g.as_mut() {
            let _ = c.kill().await;
        }
        *g = None;
        let mut s = e.state.write();
        s.name = "Stopped".into();
        s.pid = None;
        Ok(())
    }
    pub async fn restart(&self, id: &str) -> Result<()> {
        self.stop(id).await?;
        self.start(id).await
    }
    pub async fn stop_all(&self) {
        for id in self.entries.keys() {
            let _ = self.stop(id).await;
        }
    }
    pub async fn logs(&self, id: &str, n: usize) -> Result<Vec<String>> {
        self.get(id)?;
        let t = tokio::fs::read_to_string(self.log_dir.join(format!("{id}.log")))
            .await
            .unwrap_or_default();
        let mut v = t
            .lines()
            .rev()
            .take(n.min(5000))
            .map(str::to_string)
            .collect::<Vec<_>>();
        v.reverse();
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        assert_eq!(attempts(), 5);
        assert_eq!(policy(), "on_failure");
    }
}
