//! nftables rules that hand traffic to nfqws through NFQUEUE (Linux).
//!
//! winws opens its own WinDivert filter; nfqws only sees what the firewall queues to it, so the
//! service owns one table, `inet dpimech`, for as long as an nfqws engine runs. Only the first
//! packets of each connection are queued (enough for the TLS/QUIC handshake), and `bypass`
//! lets traffic flow normally if nfqws is not listening, so a crash never cuts the network.
//! Only one packet engine runs at a time (see `EngineKind::intercepts_packets`), so one queue
//! number and one table are enough.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, bail};

pub const QUEUE_NUM: u16 = 200;
/// nfqws marks the packets it sends itself; they must not be queued again.
pub const DESYNC_MARK: &str = "0x40000000";
const TABLE: &str = "dpimech";

/// Removes the table when dropped (profile stopped, engine restarted, Lab test finished).
pub struct Rules(());

impl Rules {
    /// Queues the ports the strategy filters on (`--filter-tcp` / `--filter-udp`), or HTTP,
    /// HTTPS and QUIC when it does not say.
    pub fn install(engine_args: &[String]) -> anyhow::Result<Rules> {
        let ruleset = ruleset(engine_args)?;
        // Replace, don't stack: a crashed service may have left the table behind.
        remove();
        nft(&ruleset).map_err(|e| {
            // The kernel rejects the `queue` expression when nft_queue is not available.
            if e.to_string().contains("No such file or directory") {
                anyhow::anyhow!(
                    "this kernel cannot queue packets to nfqws (modules nft_queue and nfnetlink_queue are missing): {e}"
                )
            } else {
                e.context("installing the nftables rules for nfqws")
            }
        })?;
        Ok(Rules(()))
    }
}

impl Drop for Rules {
    fn drop(&mut self) {
        remove();
    }
}

/// Deletes the table if present. Called at service start in case a crash left it.
pub fn remove() {
    remove_table(TABLE);
}

/// Also used once for the table of the old "dpimngr" service.
pub fn remove_table(name: &str) {
    let _ = nft(&format!("destroy table inet {name}\n")).or_else(|_| {
        // `destroy` needs nft 1.0.8+; `delete` fails harmlessly when the table is absent.
        nft(&format!("delete table inet {name}\n"))
    });
}

fn ruleset(args: &[String]) -> anyhow::Result<String> {
    let tcp = ports(args, "--filter-tcp")?.unwrap_or_else(|| "80, 443".into());
    let udp = ports(args, "--filter-udp")?.unwrap_or_else(|| "443".into());
    let q = QUEUE_NUM;
    let mark = DESYNC_MARK;
    Ok(format!(
        "\
table inet {TABLE} {{
    chain out {{
        type filter hook postrouting priority mangle; policy accept;
        meta mark & {mark} == 0 tcp dport {{ {tcp} }} ct original packets 1-6 queue flags bypass to {q}
        meta mark & {mark} == 0 udp dport {{ {udp} }} ct original packets 1-6 queue flags bypass to {q}
    }}
    chain in {{
        type filter hook prerouting priority filter; policy accept;
        tcp sport {{ {tcp} }} ct reply packets 1-3 queue flags bypass to {q}
    }}
}}
"
    ))
}

/// Union of every value given for `option` (`--filter-tcp=80,443` or `--filter-tcp 80`), as an
/// nftables set body. The values end up in a root firewall script, so anything but port
/// numbers and ranges is refused.
pub(crate) fn ports(args: &[String], option: &str) -> anyhow::Result<Option<String>> {
    let mut values = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = if let Some(v) = arg
            .strip_prefix(option)
            .and_then(|rest| rest.strip_prefix('='))
        {
            v.to_owned()
        } else if arg == option {
            iter.next().cloned().unwrap_or_default()
        } else {
            continue;
        };
        for part in value.split(',') {
            let (lo, hi) = part.split_once('-').unwrap_or((part, part));
            let (Ok(lo), Ok(hi)) = (lo.parse::<u16>(), hi.parse::<u16>()) else {
                bail!("{option}: \"{part}\" is not a port or port range");
            };
            if lo == 0 || lo > hi {
                bail!("{option}: \"{part}\" is not a valid port range");
            }
            let item = if lo == hi {
                lo.to_string()
            } else {
                format!("{lo}-{hi}")
            };
            if !values.contains(&item) {
                values.push(item);
            }
        }
    }
    Ok((!values.is_empty()).then(|| values.join(", ")))
}

pub(crate) fn nft(script: &str) -> anyhow::Result<()> {
    let mut child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("running nft (is nftables installed?)")?;
    child
        .stdin
        .take()
        .context("nft stdin")?
        .write_all(script.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        dpimech_core::args::split_args(s)
    }

    #[test]
    fn defaults_to_web_ports() {
        let rules = ruleset(&args("--dpi-desync=fake")).unwrap();
        assert!(
            rules.contains(
                "tcp dport { 80, 443 } ct original packets 1-6 queue flags bypass to 200"
            )
        );
        assert!(rules.contains("udp dport { 443 }"));
        assert!(rules.contains("meta mark & 0x40000000 == 0"));
    }

    #[test]
    fn collects_ports_from_every_profile_section() {
        let rules = ruleset(&args(
            "--filter-udp=50000-50100 --dpi-desync=fake --new --filter-udp 443 --new --filter-tcp=80,443 --new --filter-udp=443",
        ))
        .unwrap();
        assert!(rules.contains("udp dport { 50000-50100, 443 }"));
        assert!(rules.contains("tcp dport { 80, 443 }"));
    }

    #[test]
    fn refuses_anything_but_ports() {
        assert!(ruleset(&args("--filter-tcp=443;flush_ruleset")).is_err());
        assert!(ruleset(&args(r#""--filter-tcp=443 }""#)).is_err());
        assert!(ruleset(&args("--filter-udp=0")).is_err());
        assert!(ruleset(&args("--filter-udp=500-400")).is_err());
        assert!(ruleset(&args("--filter-tcp=70000")).is_err());
    }
}
