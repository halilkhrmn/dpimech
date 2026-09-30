//! Turns an engine + strategy into a ready-to-spawn command. Shared by profiles and the
//! Strategy Lab so both apply the same placeholders, defaults and argument policy.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, bail};
use dpimech_core::argpolicy::check_engine_args;
use dpimech_core::args::split_args;
use dpimech_core::catalog::DEFAULT_SNI;
use dpimech_core::model::EngineKind;
use dpimech_core::packages::PackageId;
use dpimech_core::paths::DataDir;
use tokio::process::Command;

use crate::packages::Packages;

/// ByeDPI's default (512 events ≈ 256 proxied connections) fills up after hours of Discord use.
const BYEDPI_DEFAULT_MAX_CONN: &str = "4096";

pub struct Launch<'a> {
    pub engine: EngineKind,
    pub args: &'a str,
    /// Local proxy port for proxy engines.
    pub port: Option<u16>,
    /// Domains written to the `{hostlist}` file.
    pub domains: &'a [String],
    /// File name (without extension) for the generated hostlist, unique per profile/test.
    pub hostlist_name: &'a str,
}

pub fn build(data: &DataDir, packages: &Packages, launch: &Launch) -> anyhow::Result<Command> {
    if PackageId::for_engine(launch.engine).is_none() {
        bail!(
            "{} support is coming in a later phase",
            launch.engine.display_name()
        );
    }
    let exe = packages.engine_path(launch.engine).with_context(|| {
        format!(
            "{} is not installed — install it from Engines",
            launch.engine.display_name()
        )
    })?;
    let engine_dir = exe.parent().context("engine path has no parent")?;
    let fake_dir = engine_dir.join("fake");
    let lists = data.lists_dir();
    std::fs::create_dir_all(&lists)?;

    let hostlist = if launch.args.contains("{hostlist}") {
        if launch.domains.is_empty() {
            bail!(
                "this strategy needs at least one domain (add Discord, YouTube, … to the profile)"
            );
        }
        Some(write_hostlist(
            &lists,
            launch.hostlist_name,
            launch.domains,
        )?)
    } else {
        None
    };

    let user_args = resolve(launch.args, &fake_dir, &lists, hostlist.as_deref());
    // Re-checked at every launch: the config file may have been edited by hand.
    check_engine_args(launch.engine, &user_args, &[&lists, &fake_dir])
        .map_err(anyhow::Error::msg)?;

    let mut args: Vec<String> = Vec::new();
    if launch.engine == EngineKind::ByeDpi {
        let port = launch.port.context("ByeDPI needs a local port")?;
        args.extend([
            "-i".into(),
            "127.0.0.1".into(),
            "-p".into(),
            port.to_string(),
        ]);
        if !sets_max_conn(&user_args) {
            args.extend(["-c".into(), BYEDPI_DEFAULT_MAX_CONN.into()]);
        }
    }
    #[cfg(target_os = "linux")]
    if launch.engine == EngineKind::ZapretNfqws {
        use crate::nfqueue::{DESYNC_MARK, QUEUE_NUM};
        args.extend([
            format!("--qnum={QUEUE_NUM}"),
            format!("--dpi-desync-fwmark={DESYNC_MARK}"),
        ]);
    }
    if launch.engine == EngineKind::SpoofDpi {
        let port = launch.port.context("SpoofDPI needs a local port")?;
        // --clean: never read a spoofdpi.toml from /etc or a home directory.
        args.extend([
            "--app-mode".into(),
            "socks5".into(),
            "--listen-addr".into(),
            format!("127.0.0.1:{port}"),
            "--no-tui".into(),
            "--clean".into(),
        ]);
    }
    if launch.engine == EngineKind::ZapretTpws {
        let port = launch.port.context("tpws needs a local port")?;
        args.extend([
            "--bind-addr=127.0.0.1".into(),
            format!("--port={port}"),
            "--socks".into(),
        ]);
    }
    args.extend(user_args);

    let mut cmd = Command::new(&exe);
    cmd.args(&args)
        .current_dir(engine_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    crate::job::contain(&mut cmd);
    Ok(cmd)
}

/// Firewall state an engine needs while it runs; dropping it undoes the change.
#[cfg(target_os = "linux")]
pub type EngineRules = crate::nfqueue::Rules;
#[cfg(not(target_os = "linux"))]
pub type EngineRules = ();

/// Installs what `cmd` needs outside the process itself: nfqws only sees traffic that
/// nftables queues to it. Keep the result alive for as long as the engine runs.
pub fn engine_rules(engine: EngineKind, cmd: &Command) -> anyhow::Result<Option<EngineRules>> {
    #[cfg(target_os = "linux")]
    if engine == EngineKind::ZapretNfqws {
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        return crate::nfqueue::Rules::install(&args).map(Some);
    }
    let _ = (engine, cmd);
    Ok(None)
}

/// Checks arguments without needing the engine installed (used when a profile is saved).
pub fn validate(data: &DataDir, engine: EngineKind, args: &str) -> Result<(), String> {
    let lists = data.lists_dir();
    let fake = data
        .package_dir(PackageId::Zapret)
        .join("current")
        .join("fake");
    let hostlist = lists.join("check.txt");
    let resolved = resolve(args, &fake, &lists, Some(&hostlist));
    check_engine_args(engine, &resolved, &[&lists, &fake])
}

/// Splits first, then substitutes per token, so paths with spaces stay one argument.
fn resolve(args: &str, fake: &Path, lists: &Path, hostlist: Option<&Path>) -> Vec<String> {
    let fake = fake.display().to_string();
    let lists = lists.display().to_string();
    let hostlist = hostlist
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    split_args(args)
        .into_iter()
        .map(|a| {
            a.replace("{fake}", &fake)
                .replace("{lists}", &lists)
                .replace("{hostlist}", &hostlist)
                .replace("{sni}", DEFAULT_SNI)
        })
        .collect()
}

fn write_hostlist(lists: &Path, name: &str, domains: &[String]) -> anyhow::Result<PathBuf> {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = lists.join(format!("{safe}.txt"));
    let mut body = String::new();
    for d in domains {
        let d = d.trim().trim_start_matches("*.").to_ascii_lowercase();
        // Hostnames only: this file is read by a SYSTEM process.
        if !d.is_empty()
            && d.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            body.push_str(&d);
            body.push('\n');
        }
    }
    if body.is_empty() {
        bail!("no valid domains");
    }
    std::fs::write(&path, body)?;
    Ok(path)
}

/// True if the user already chose a connection limit (`-c N`, `-cN`, `--max-conn[=N]`).
pub fn sets_max_conn(args: &[String]) -> bool {
    args.iter().any(|a| {
        a == "-c"
            || a.starts_with("--max-conn")
            || (a.len() > 2 && a.starts_with("-c") && a[2..].chars().all(|c| c.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::sets_max_conn;

    fn args(s: &str) -> Vec<String> {
        dpimech_core::args::split_args(s)
    }

    #[test]
    fn detects_user_connection_limit() {
        assert!(sets_max_conn(&args("-r 1+s -c 100")));
        assert!(sets_max_conn(&args("-c100")));
        assert!(sets_max_conn(&args("--max-conn=200")));
        assert!(!sets_max_conn(&args("-r 1+s")));
        // -cX with non-digits is some other option, not the connection limit.
        assert!(!sets_max_conn(&args("-cache")));
    }
}
