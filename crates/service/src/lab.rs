//! Strategy Lab: tries strategies against real sites and reports which ones work.
//!
//! - Proxy engines (ByeDPI, tpws) run up to [`PARALLEL`] at once on private ports; requests go through
//!   them as `socks5h` so DNS is resolved by the engine (immune to DNS spoofing).
//! - WinDivert engines (winws, GoodbyeDPI) act on the whole machine, so they run one at a
//!   time, scoped to the test domains via `{hostlist}`.
//! - A baseline run without any bypass shows which sites are actually blocked.

use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use dpimech_core::catalog::{self, SourceFormat};
use dpimech_core::ipc::Event;
use dpimech_core::lab::{IspInfo, LabRequest, LabResult, LabStrategy};
use dpimech_core::model::EngineKind;
use dpimech_core::paths::DataDir;
use futures_util::future::join_all;
use serde::Deserialize;
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;

use crate::launch::{self, Launch};
use crate::logs::LogBus;
use crate::packages::Packages;

const SOURCE: &str = "lab";
const PARALLEL: usize = 4;
const FIRST_PORT: u16 = 30150;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// WinDivert needs a moment to load its driver and start filtering.
const WINDIVERT_WARMUP: Duration = Duration::from_millis(2000);

#[derive(Clone)]
pub struct Lab {
    data: Arc<DataDir>,
    packages: Packages,
    events: broadcast::Sender<Event>,
    logs: Arc<LogBus>,
    http: reqwest::Client,
    task: Arc<Mutex<Option<JoinHandle<()>>>>,
    isp: Arc<StdMutex<Option<IspInfo>>>,
}

impl Lab {
    pub fn new(
        data: Arc<DataDir>,
        packages: Packages,
        events: broadcast::Sender<Event>,
        logs: Arc<LogBus>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("dpimech/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("http client");
        Self {
            data,
            packages,
            events,
            logs,
            http,
            task: Arc::default(),
            isp: Arc::default(),
        }
    }

    pub async fn is_running(&self) -> bool {
        self.task
            .lock()
            .await
            .as_ref()
            .is_some_and(|t| !t.is_finished())
    }

    /// Built-in strategies plus the cached online lists, ISP presets first.
    pub fn strategies(&self, engine: EngineKind) -> Vec<LabStrategy> {
        let mut out: Vec<LabStrategy> = catalog::builtin_strategies(engine)
            .iter()
            .map(|s| LabStrategy {
                name: s.name.to_owned(),
                args: catalog::adapt_args(engine, s.args),
                source: catalog::STANDARD_SET.to_owned(),
                origin: String::new(),
                recommended: false,
            })
            .collect();
        if let Ok(text) = std::fs::read_to_string(self.cache_file(engine))
            && let Ok(cached) = serde_json::from_str::<Vec<LabStrategy>>(&text)
        {
            for s in cached {
                if !out.iter().any(|o| o.args == s.args) {
                    out.push(s);
                }
            }
        }
        let preset_keys = self
            .isp
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|i| i.known.as_deref())
            .and_then(|name| catalog::ISPS.iter().find(|isp| isp.name == name))
            .map(|isp| isp.preset_keys)
            .unwrap_or_default();
        for s in &mut out {
            let name = s.name.to_lowercase();
            s.recommended = preset_keys.iter().any(|k| name.contains(k));
        }
        out.sort_by_key(|s| !s.recommended);
        out
    }

    pub async fn refresh(&self, engine: EngineKind) -> anyhow::Result<Vec<LabStrategy>> {
        let mut fetched = Vec::new();
        for source in catalog::online_sources(engine) {
            let text = self
                .http
                .get(source.url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .with_context(|| format!("downloading {} ({})", source.label, source.origin))?
                .text()
                .await?;
            for (name, args) in catalog::parse_source(source.format, &text) {
                let mut name = match source.format {
                    SourceFormat::ArgsPerLine => format!("Community {name}"),
                    SourceFormat::NameColonArgs => name,
                };
                // Some lists reuse a name for different settings; keep rows distinguishable.
                let base = name.clone();
                let mut n = 2;
                while fetched.iter().any(|s: &LabStrategy| s.name == name) {
                    name = format!("{base} ({n})");
                    n += 1;
                }
                fetched.push(LabStrategy {
                    name,
                    args,
                    source: source.label.to_owned(),
                    origin: source.origin.to_owned(),
                    recommended: false,
                });
            }
        }
        let file = self.cache_file(engine);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&file, serde_json::to_string_pretty(&fetched)?)?;
        self.logs.info(
            SOURCE,
            format!(
                "{} online strategies for {}",
                fetched.len(),
                engine.display_name()
            ),
        );
        Ok(self.strategies(engine))
    }

    pub async fn detect_isp(&self) -> anyhow::Result<IspInfo> {
        #[derive(Deserialize)]
        struct IpWhoIs {
            success: bool,
            #[serde(default)]
            country_code: String,
            connection: Option<Connection>,
        }
        #[derive(Deserialize)]
        struct Connection {
            asn: Option<u32>,
            #[serde(default)]
            isp: String,
            #[serde(default)]
            org: String,
        }
        let reply: IpWhoIs = self
            .http
            .get("https://ipwho.is/?fields=success,country_code,connection")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if !reply.success {
            bail!("ISP lookup failed");
        }
        let conn = reply
            .connection
            .context("ISP lookup returned no provider")?;
        let provider = if conn.isp.is_empty() {
            conn.org
        } else {
            conn.isp
        };
        let known = catalog::match_isp(conn.asn, &provider).map(|i| i.name.to_owned());
        let info = IspInfo {
            provider,
            asn: conn.asn,
            country: reply.country_code,
            known,
        };
        *self.isp.lock().unwrap() = Some(info.clone());
        Ok(info)
    }

    pub async fn start(&self, request: LabRequest) -> anyhow::Result<()> {
        if request.domains.is_empty() {
            bail!("choose at least one site to test");
        }
        if request.strategies.is_empty() {
            bail!("no strategies selected");
        }
        let mut task = self.task.lock().await;
        if task.as_ref().is_some_and(|t| !t.is_finished()) {
            bail!("a test is already running");
        }
        let this = self.clone();
        *task = Some(tokio::spawn(async move {
            let error = this.run(request).await.err().map(|e| format!("{e:#}"));
            if let Some(e) = &error {
                this.logs.error(SOURCE, e);
            }
            let _ = this.events.send(Event::LabFinished {
                cancelled: false,
                error,
            });
        }));
        Ok(())
    }

    pub async fn cancel(&self) {
        // Dropping the task drops the engine children, which kills them (kill_on_drop).
        if let Some(task) = self.task.lock().await.take()
            && !task.is_finished()
        {
            task.abort();
            let _ = self.events.send(Event::LabFinished {
                cancelled: true,
                error: None,
            });
        }
    }

    fn cache_file(&self, engine: EngineKind) -> std::path::PathBuf {
        self.data
            .root
            .join("strategies")
            .join(format!("{}.json", engine.slug()))
    }

    async fn run(&self, request: LabRequest) -> anyhow::Result<()> {
        let hosts = if request.probes.is_empty() {
            request.domains.clone()
        } else {
            request.probes.clone()
        };
        let repeats = u32::from(request.repeats.clamp(1, 5));
        let total = request.strategies.len() as u32 + 1;
        let mut done = 0;
        self.logs.info(
            SOURCE,
            format!(
                "testing {} strategies for {} on {} site(s)",
                request.strategies.len(),
                request.engine.display_name(),
                hosts.len()
            ),
        );

        crate::dnscheck::log_mismatches(&self.logs, SOURCE, &hosts).await;
        let direct = direct_client()?;
        let baseline = check_sites(&direct, &hosts, repeats).await;
        self.logs.info(
            SOURCE,
            format!(
                "without a bypass: {}/{} site(s) open{}",
                baseline.ok,
                baseline.total,
                failed_list(&baseline.failed_domains)
            ),
        );
        let best: StdMutex<Option<LabResult>> = StdMutex::new(None);
        done += 1;
        self.emit(
            done,
            total,
            LabResult {
                strategy: None,
                ..baseline
            },
        );

        let strategies: Vec<LabStrategy> = request
            .strategies
            .into_iter()
            .map(|mut s| {
                s.args = catalog::scope_to_hostlist(request.engine, &s.args);
                s
            })
            .collect();

        if request.engine.is_proxy() {
            let queue = Arc::new(StdMutex::new(VecDeque::from(strategies)));
            let done = Arc::new(std::sync::atomic::AtomicU32::new(done));
            let best = &best;
            let workers = (0..PARALLEL).map(|w| {
                let queue = queue.clone();
                let done = done.clone();
                let hosts = hosts.clone();
                let domains = request.domains.clone();
                async move {
                    let port = FIRST_PORT + w as u16;
                    loop {
                        let Some(strategy) = queue.lock().unwrap().pop_front() else {
                            break;
                        };
                        let result = self
                            .try_proxy_strategy(
                                request.engine,
                                &strategy,
                                port,
                                &domains,
                                &hosts,
                                repeats,
                            )
                            .await;
                        let n = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        self.record(best, &result);
                        self.emit(n, total, result);
                    }
                }
            });
            join_all(workers).await;
        } else {
            for strategy in strategies {
                let result = self
                    .try_packet_strategy(
                        &strategy,
                        request.engine,
                        &request.domains,
                        &hosts,
                        &direct,
                        repeats,
                    )
                    .await;
                done += 1;
                self.record(&best, &result);
                self.emit(done, total, result);
            }
        }
        match best.into_inner().unwrap() {
            Some(b) if b.ok > 0 => {
                let strategy = b
                    .strategy
                    .as_ref()
                    .map(|s| s.args.as_str())
                    .unwrap_or_default();
                let text = format!(
                    "best strategy opens {}/{} site(s), {} ms: {strategy}{}",
                    b.ok,
                    b.total,
                    b.avg_ms,
                    failed_list(&b.failed_domains)
                );
                if b.ok < b.total {
                    self.logs.warn(SOURCE, text);
                } else {
                    self.logs.info(SOURCE, text);
                }
            }
            _ => self
                .logs
                .warn(SOURCE, "no strategy opened any of the sites"),
        }
        self.logs.info(SOURCE, "test finished");
        Ok(())
    }

    /// Detailed log line for one strategy, and keeps the best result for the summary.
    fn record(&self, best: &StdMutex<Option<LabResult>>, result: &LabResult) {
        self.logs.debug(SOURCE, || {
            let strategy = result
                .strategy
                .as_ref()
                .map(|s| format!("{} ({})", s.name, s.args))
                .unwrap_or_default();
            match &result.error {
                Some(e) => format!("{strategy}: could not start: {e}"),
                None => format!(
                    "{strategy}: {}/{} site(s), {} ms{}",
                    result.ok,
                    result.total,
                    result.avg_ms,
                    failed_list(&result.failed_domains)
                ),
            }
        });
        let mut best = best.lock().unwrap();
        if best.as_ref().is_none_or(|b| result.score() > b.score()) {
            *best = Some(result.clone());
        }
    }

    fn emit(&self, done: u32, total: u32, result: LabResult) {
        let _ = self.events.send(Event::LabProgress {
            done,
            total,
            result: Some(result),
        });
    }

    async fn try_proxy_strategy(
        &self,
        engine: EngineKind,
        strategy: &LabStrategy,
        port: u16,
        domains: &[String],
        hosts: &[String],
        repeats: u32,
    ) -> LabResult {
        let failed = |error: String| LabResult {
            strategy: Some(strategy.clone()),
            ok: 0,
            total: hosts.len() as u32 * repeats,
            avg_ms: 0,
            failed_domains: hosts.to_vec(),
            error: Some(error),
        };
        let launch = Launch {
            engine,
            args: &strategy.args,
            port: Some(port),
            domains,
            hostlist_name: "lab",
        };
        let mut cmd = match launch::build(&self.data, &self.packages, &launch) {
            Ok(cmd) => cmd,
            Err(e) => return failed(format!("{e:#}")),
        };
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => return failed(format!("failed to start: {e}")),
        };
        crate::job::adopt(&child);

        if !wait_for_port(port, &mut child).await {
            let _ = child.kill().await;
            return failed("engine did not start (invalid arguments?)".to_owned());
        }
        let result = match socks_client(port) {
            Ok(client) => check_sites(&client, hosts, repeats).await,
            Err(e) => failed(e.to_string()),
        };
        let _ = child.kill().await;
        LabResult {
            strategy: Some(strategy.clone()),
            ..result
        }
    }

    async fn try_packet_strategy(
        &self,
        strategy: &LabStrategy,
        engine: EngineKind,
        domains: &[String],
        hosts: &[String],
        client: &reqwest::Client,
        repeats: u32,
    ) -> LabResult {
        let failed = |error: String| LabResult {
            strategy: Some(strategy.clone()),
            ok: 0,
            total: hosts.len() as u32 * repeats,
            avg_ms: 0,
            failed_domains: hosts.to_vec(),
            error: Some(error),
        };
        let launch = Launch {
            engine,
            args: &strategy.args,
            port: None,
            domains,
            hostlist_name: "lab",
        };
        let mut cmd = match launch::build(&self.data, &self.packages, &launch) {
            Ok(cmd) => cmd,
            Err(e) => return failed(format!("{e:#}")),
        };
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let _rules = match launch::engine_rules(engine, &cmd) {
            Ok(rules) => rules,
            Err(e) => return failed(format!("{e:#}")),
        };
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.raw_os_error() == Some(740) => {
                return failed("needs administrator rights (installed service)".to_owned());
            }
            Err(e) => return failed(format!("failed to start: {e}")),
        };
        crate::job::adopt(&child);
        tokio::time::sleep(WINDIVERT_WARMUP).await;
        if let Ok(Some(status)) = child.try_wait() {
            return failed(format!("engine exited ({status}) — invalid arguments?"));
        }
        let result = check_sites(client, hosts, repeats).await;
        let _ = child.kill().await;
        // Let WinDivert release its handle before the next engine loads.
        tokio::time::sleep(Duration::from_millis(500)).await;
        LabResult {
            strategy: Some(strategy.clone()),
            ..result
        }
    }
}

fn direct_client() -> reqwest::Result<reqwest::Client> {
    base_client().build()
}

fn socks_client(port: u16) -> reqwest::Result<reqwest::Client> {
    // socks5h: the engine resolves DNS, like a browser configured with this proxy.
    base_client()
        .proxy(reqwest::Proxy::all(format!("socks5h://127.0.0.1:{port}"))?)
        .build()
}

fn base_client() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36",
        )
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(REQUEST_TIMEOUT)
        // A redirect already proves the TLS handshake got through.
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(0)
}

/// Any HTTP response counts as success: it means the TLS handshake got past DPI.
async fn check_sites(client: &reqwest::Client, hosts: &[String], repeats: u32) -> LabResult {
    let per_host = hosts.iter().map(|host| async move {
        let mut ok = 0u32;
        let mut ms = 0u64;
        for _ in 0..repeats {
            let started = Instant::now();
            if client.get(format!("https://{host}/")).send().await.is_ok() {
                ok += 1;
                ms += started.elapsed().as_millis() as u64;
            }
        }
        (host.clone(), ok, ms)
    });
    let results = join_all(per_host).await;
    let ok: u32 = results.iter().map(|r| r.1).sum();
    let ms: u64 = results.iter().map(|r| r.2).sum();
    LabResult {
        strategy: None,
        ok,
        total: hosts.len() as u32 * repeats,
        avg_ms: if ok == 0 {
            0
        } else {
            (ms / u64::from(ok)) as u32
        },
        failed_domains: results
            .into_iter()
            .filter(|r| r.1 < repeats)
            .map(|r| r.0)
            .collect(),
        error: None,
    }
}

async fn wait_for_port(port: u16, child: &mut tokio::process::Child) -> bool {
    for _ in 0..30 {
        if let Ok(Some(_)) = child.try_wait() {
            return false;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// ` — failed: a, b` or nothing.
fn failed_list(domains: &[String]) -> String {
    if domains.is_empty() {
        String::new()
    } else {
        format!(" — failed: {}", domains.join(", "))
    }
}
