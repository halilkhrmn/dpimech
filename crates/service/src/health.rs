//! Active health checks for proxy engines.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

const INTERVAL: Duration = Duration::from_secs(20);
const TIMEOUT: Duration = Duration::from_secs(3);
/// One slow answer can be a hiccup; two in a row means the proxy is stuck.
const FAILURES_BEFORE_RESTART: u32 = 2;

/// Periodically performs a SOCKS5 greeting against `127.0.0.1:port` and reports on `stall`
/// once the engine stops answering. Runs until aborted.
pub async fn probe_socks(port: u16, stall: mpsc::Sender<String>) {
    let mut failures = 0;
    loop {
        tokio::time::sleep(INTERVAL).await;
        match tokio::time::timeout(TIMEOUT, socks_greeting(port)).await {
            Ok(Ok(())) => failures = 0,
            result => {
                failures += 1;
                let why = match result {
                    Ok(Err(e)) => e.to_string(),
                    _ => "no answer within 3 s".to_owned(),
                };
                tracing::debug!("health probe on port {port} failed: {why}");
                if failures >= FAILURES_BEFORE_RESTART {
                    let _ = stall
                        .send(format!("health check failed {failures}× ({why})"))
                        .await;
                    failures = 0;
                }
            }
        }
    }
}

/// `05 01 00` (SOCKS5, one method, no auth) must be answered with `05 00`.
async fn socks_greeting(port: u16) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    stream.write_all(&[5, 1, 0]).await?;
    let mut reply = [0u8; 2];
    stream.read_exact(&mut reply).await?;
    if reply == [5, 0] {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("unexpected reply {reply:?}")))
    }
}

// ---------------------------------------------------------------------------------------------
// Connection monitor: periodic requests to the profile's sites through the engine.
// ---------------------------------------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use dpimech_core::model::ConnectionHealth;
use tokio::sync::Notify;

const SITE_TIMEOUT: Duration = Duration::from_secs(8);
/// Long enough for the engine to open its port and per-app routes to be in place.
const FIRST_CHECK: Duration = Duration::from_secs(4);
/// After a bad check, look again soon instead of waiting a full interval.
const CONFIRM_AFTER: Duration = Duration::from_secs(30);
const USUAL_SAMPLES: usize = 10;
/// Restarting more often than this without improvement means the strategy itself stopped
/// working; the monitor then only warns.
const MAX_SLOW_RESTARTS: usize = 2;
const SLOW_RESTART_WINDOW: Duration = Duration::from_secs(30 * 60);

/// Survives engine restarts so "usual" latency and loop protection are per profile run.
#[derive(Default)]
pub struct MonitorHistory {
    usual: VecDeque<u32>,
    slow_restarts: VecDeque<Instant>,
}

impl MonitorHistory {
    fn usual_ms(&self) -> u32 {
        // Two samples are enough to start judging; before that we are still learning.
        if self.usual.len() < 2 {
            return 0;
        }
        median(self.usual.iter().copied().collect())
    }

    /// Records an automatic restart; returns false once restarts stopped helping.
    pub fn allow_restart(&mut self) -> bool {
        let now = Instant::now();
        while self
            .slow_restarts
            .front()
            .is_some_and(|t| now - *t > SLOW_RESTART_WINDOW)
        {
            self.slow_restarts.pop_front();
        }
        if self.slow_restarts.len() >= MAX_SLOW_RESTARTS {
            return false;
        }
        self.slow_restarts.push_back(now);
        true
    }
}

pub enum MonitorEvent {
    /// The check right after the engine started.
    First(ConnectionHealth, Vec<String>),
    /// A normal (or first bad, still unconfirmed) check.
    Report(ConnectionHealth),
    /// Two bad checks in a row.
    Slow(ConnectionHealth, String),
}

/// Checks `sites` every `interval` (or when `check_now` fires) and reports on `events`.
/// Requests go through the engine's SOCKS port for proxy engines, directly otherwise.
pub async fn monitor_sites(
    sites: Vec<String>,
    proxy_port: Option<u16>,
    interval: Duration,
    history: Arc<Mutex<MonitorHistory>>,
    check_now: Arc<Notify>,
    events: mpsc::Sender<MonitorEvent>,
) {
    let Ok(client) = client(proxy_port) else {
        return;
    };
    // First check right after start: a setting that does not work should say so at once.
    let mut wait = FIRST_CHECK;
    let mut bad_streak = 0u32;
    let mut first = true;
    loop {
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = check_now.notified() => {}
        }
        let (latency_ms, failed) = measure(&client, &sites).await;
        let ok = (sites.len() - failed.len()) as u32;
        let total = sites.len() as u32;
        let usual_ms = history.lock().unwrap().usual_ms();
        let too_few = ok * 2 < total;
        let too_slow = usual_ms > 0 && latency_ms > (usual_ms * 3).max(usual_ms + 500);
        let bad = too_few || too_slow;
        if bad {
            bad_streak += 1;
        } else {
            bad_streak = 0;
            let mut h = history.lock().unwrap();
            h.usual.push_back(latency_ms);
            if h.usual.len() > USUAL_SAMPLES {
                h.usual.pop_front();
            }
        }
        let health = ConnectionHealth {
            latency_ms,
            usual_ms,
            ok,
            total,
            checked_unix: crate::logs::now_ms() / 1000,
            slow: bad,
            advice: None,
        };
        let event = if std::mem::take(&mut first) {
            MonitorEvent::First(health, failed)
        } else if bad_streak >= 2 {
            bad_streak = 0;
            let reason = if too_few {
                format!("only {ok}/{total} sites answered")
            } else {
                format!("responses took {latency_ms} ms (usually {usual_ms} ms)")
            };
            MonitorEvent::Slow(health, reason)
        } else {
            MonitorEvent::Report(health)
        };
        if events.send(event).await.is_err() {
            return;
        }
        wait = if bad_streak == 1 {
            CONFIRM_AFTER
        } else {
            interval
        };
    }
}

fn client(proxy_port: Option<u16>) -> reqwest::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(SITE_TIMEOUT)
        .connect_timeout(SITE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        // Fresh connections every time: a pooled one would hide a stalled engine.
        .pool_max_idle_per_host(0);
    if let Some(port) = proxy_port {
        builder = builder.proxy(reqwest::Proxy::all(format!("socks5h://127.0.0.1:{port}"))?);
    }
    builder.build()
}

/// Median response time of the sites that answered, and the sites that did not.
async fn measure(client: &reqwest::Client, sites: &[String]) -> (u32, Vec<String>) {
    let checks = sites.iter().map(|site| async move {
        let started = Instant::now();
        let opened = site_opens(client, site).await;
        (site, opened.then(|| started.elapsed().as_millis() as u32))
    });
    let results = futures_util::future::join_all(checks).await;
    let times: Vec<u32> = results.iter().filter_map(|r| r.1).collect();
    let failed = results
        .iter()
        .filter(|r| r.1.is_none())
        .map(|r| r.0.clone())
        .collect();
    (if times.is_empty() { 0 } else { median(times) }, failed)
}

/// Enough of the page to know data keeps flowing; big pages would only slow the check down.
const BODY_SAMPLE: usize = 64 * 1024;

/// Requests `https://{host}/` and reads the start of the page. Response headers alone are not
/// enough: DPI often lets the TLS handshake through and resets the connection once data flows,
/// which is exactly the "connected but nothing loads" case.
pub async fn site_opens(client: &reqwest::Client, host: &str) -> bool {
    let Ok(mut response) = client.get(format!("https://{host}/")).send().await else {
        return false;
    };
    let mut read = 0;
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                read += chunk.len();
                if read >= BODY_SAMPLE {
                    return true;
                }
            }
            Ok(None) => return true,
            Err(_) => return false,
        }
    }
}

fn median(mut v: Vec<u32>) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}
