//! Spots DNS blocking: some providers (e.g. in Turkey) answer blocked names with a wrong
//! address. No engine can fix that, because the app then connects to the wrong server. The
//! check compares the system's answer with Cloudflare's DNS-over-HTTPS answer.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;

use crate::logs::LogBus;

/// Enough to cover a pack's main names without slowing the Lab down.
const MAX_HOSTS: usize = 4;
const TIMEOUT: Duration = Duration::from_secs(4);

/// Logs one line per checked host: a warning when the answers point at different networks,
/// otherwise a detailed-log line.
pub async fn log_mismatches(logs: &Arc<LogBus>, source: &str, hosts: &[String]) {
    let Ok(client) = reqwest::Client::builder().timeout(TIMEOUT).build() else {
        return;
    };
    let mut names: Vec<&str> = Vec::new();
    for host in hosts {
        let name = host.split('/').next().unwrap_or(host);
        let plain = name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
        if plain && !names.contains(&name) && name.parse::<IpAddr>().is_err() {
            names.push(name);
        }
    }
    names.truncate(MAX_HOSTS);
    let checks = names.into_iter().map(|name| {
        let client = client.clone();
        async move { (name, system(name).await, doh(&client, name).await) }
    });
    for (name, system, doh) in join_all(checks).await {
        let Some(doh) = doh.filter(|d| !d.is_empty()) else {
            logs.debug(source, || {
                format!("DNS: could not ask 1.1.1.1 about {name}; not checked")
            });
            continue;
        };
        match system {
            None => logs.warn(
                source,
                format!(
                    "DNS: {name} does not resolve with your DNS, but does with 1.1.1.1 ({}) — your provider may block it by DNS; set your DNS to 1.1.1.1",
                    list(&doh)
                ),
            ),
            Some(sys) if different_networks(&sys, &doh) => logs.warn(
                source,
                format!(
                    "DNS: {name} → {} with your DNS, {} with 1.1.1.1 — if it stays blocked, your provider may block it by DNS; set your DNS to 1.1.1.1",
                    list(&sys),
                    list(&doh)
                ),
            ),
            Some(sys) => logs.debug(source, || format!("DNS: {name} → {} (matches 1.1.1.1)", list(&sys))),
        }
    }
}

async fn system(name: &str) -> Option<BTreeSet<IpAddr>> {
    let found = tokio::time::timeout(TIMEOUT, tokio::net::lookup_host((name, 443)))
        .await
        .ok()?
        .ok()?;
    let ips: BTreeSet<IpAddr> = found.map(|a| a.ip()).filter(IpAddr::is_ipv4).collect();
    (!ips.is_empty()).then_some(ips)
}

/// A records from Cloudflare over HTTPS, addressed by IP so a blocked DNS name cannot stop it.
async fn doh(client: &reqwest::Client, name: &str) -> Option<BTreeSet<IpAddr>> {
    let reply: serde_json::Value = client
        // Only plain host names get here (letters, digits, dots, dashes).
        .get(format!("https://1.1.1.1/dns-query?name={name}&type=A"))
        .header("accept", "application/dns-json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    Some(parse_doh(&reply))
}

fn parse_doh(reply: &serde_json::Value) -> BTreeSet<IpAddr> {
    reply["Answer"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["type"] == 1)
        .filter_map(|a| a["data"].as_str()?.parse().ok())
        .collect()
}

/// Big CDNs hand out different addresses per location, so only a completely different
/// network (no shared /16) counts as suspicious.
fn different_networks(a: &BTreeSet<IpAddr>, b: &BTreeSet<IpAddr>) -> bool {
    let net = |ip: &IpAddr| match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            (o[0], o[1])
        }
        IpAddr::V6(_) => (0, 0),
    };
    let nets: BTreeSet<_> = b.iter().map(net).collect();
    !a.iter().any(|ip| nets.contains(&net(ip)))
}

fn list(ips: &BTreeSet<IpAddr>) -> String {
    ips.iter()
        .take(3)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ips: &[&str]) -> BTreeSet<IpAddr> {
        ips.iter().map(|s| s.parse().unwrap()).collect()
    }

    #[test]
    fn only_a_different_network_is_suspicious() {
        let cloudflare = set(&["162.159.128.233", "162.159.135.232"]);
        assert!(!different_networks(&set(&["162.159.137.232"]), &cloudflare));
        assert!(different_networks(&set(&["195.175.254.2"]), &cloudflare));
    }

    #[test]
    fn reads_cloudflare_json() {
        let reply = serde_json::json!({
            "Status": 0,
            "Answer": [
                {"name": "discord.com", "type": 5, "data": "alias.example."},
                {"name": "discord.com", "type": 1, "data": "162.159.128.233"}
            ]
        });
        assert_eq!(parse_doh(&reply), set(&["162.159.128.233"]));
    }
}
