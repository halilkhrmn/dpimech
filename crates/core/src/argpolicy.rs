//! Allowlist for engine arguments.
//!
//! Engines run as SYSTEM/root, and profile arguments come from any local user through the IPC
//! pipe. Options that write files, read arbitrary files, or change where a proxy listens would
//! turn a profile into a privilege-escalation or exposure path, so arguments are parsed the way
//! the engine's getopt would (long-option prefixes, `--opt=value`, clustered short flags,
//! optional values) and every option is checked against a per-engine table. Unknown options are
//! rejected, so a new engine release cannot silently introduce an unsafe flag.

use std::path::Path;

use crate::model::EngineKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arg {
    None,
    Required,
    /// getopt `optional_argument`: only `--opt=value` / `-ovalue`, never the next token.
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    Allow,
    /// Chosen by DPIMech (listen address/port); users may not override it.
    Managed,
    /// Writes files, forks away or exits; never allowed.
    Deny,
    /// ByeDPI style: `:inline text` or a file in a readable directory.
    InlineOrFile,
    /// A plain file path in a readable directory.
    File,
    /// zapret payload: `0xHEX`, `!…`, or `[+ofs]@file` with the file in a readable directory.
    PayloadOrFile,
    /// Only `-` (stdout).
    StdoutOnly,
    /// zapret `--debug`: a level is fine, `@file` / `syslog` are not.
    DebugLevel,
}

struct Opt {
    short: Option<char>,
    long: &'static str,
    arg: Arg,
    policy: Policy,
}

const fn o(short: char, long: &'static str, arg: Arg, policy: Policy) -> Opt {
    Opt {
        short: Some(short),
        long,
        arg,
        policy,
    }
}

const fn l(long: &'static str, arg: Arg, policy: Policy) -> Opt {
    Opt {
        short: None,
        long,
        arg,
        policy,
    }
}

use Arg::{None as N, Optional as Opt_, Required as R};
use Policy::*;

/// ciadpi 0.17 (`ciadpi --help`).
const BYEDPI: &[Opt] = &[
    o('i', "ip", R, Managed),
    o('p', "port", R, Managed),
    o('c', "max-conn", R, Allow),
    o('N', "no-domain", N, Allow),
    o('U', "no-udp", N, Allow),
    o('I', "conn-ip", R, Allow),
    o('b', "buf-size", R, Allow),
    o('x', "debug", R, Allow),
    o('g', "def-ttl", R, Allow),
    o('A', "auto", R, Allow),
    o('L', "auto-mode", R, Allow),
    o('u', "cache-ttl", R, Allow),
    o('y', "cache-dump", R, StdoutOnly),
    o('T', "timeout", R, Allow),
    o('K', "proto", R, Allow),
    o('H', "hosts", R, InlineOrFile),
    o('j', "ipset", R, InlineOrFile),
    o('V', "pf", R, Allow),
    o('R', "round", R, Allow),
    o('s', "split", R, Allow),
    o('d', "disorder", R, Allow),
    o('o', "oob", R, Allow),
    o('q', "disoob", R, Allow),
    o('f', "fake", R, Allow),
    o('n', "fake-sni", R, Allow),
    o('t', "ttl", R, Allow),
    o('O', "fake-offset", R, Allow),
    o('l', "fake-data", R, InlineOrFile),
    o('Q', "fake-tls-mod", R, Allow),
    o('e', "oob-data", R, Allow),
    o('M', "mod-http", R, Allow),
    o('r', "tlsrec", R, Allow),
    o('m', "tlsminor", R, Allow),
    o('a', "udp-fake", R, Allow),
];

/// winws-only: WinDivert filters (zapret v72).
const WINWS_FILTERS: &[Opt] = &[
    l("wf-iface", R, Allow),
    l("wf-l3", R, Allow),
    l("wf-tcp", R, Allow),
    l("wf-udp", R, Allow),
    l("wf-tcp-in", R, Allow),
    l("wf-tcp-out", R, Allow),
    l("wf-udp-in", R, Allow),
    l("wf-udp-out", R, Allow),
    l("wf-raw", R, PayloadOrFile),
    l("wf-raw-part", R, PayloadOrFile),
    l("wf-dup-check", Opt_, Allow),
    l("wf-save", R, Deny),
    l("ssid-filter", R, Allow),
    l("nlm-filter", R, Allow),
    l("nlm-list", Opt_, Allow),
];

/// nfqws-only (Linux). The queue number and the mark that keeps nfqws' own packets out of
/// the queue must match the nftables rules DPIMech installs. `--user`/`--uid` would change
/// which account the root-started engine drops to.
const NFQWS_ONLY: &[Opt] = &[
    l("qnum", R, Managed),
    l("dpi-desync-fwmark", R, Managed),
    l("fwmark", R, Managed),
    l("user", R, Deny),
    l("uid", R, Deny),
    l("bind-fix4", N, Allow),
    l("bind-fix6", N, Allow),
];

/// Desync engine shared by winws and nfqws (winws is the Windows port of nfqws).
const ZAPRET_COMMON: &[Opt] = &[
    // profiles and filters
    l("new", N, Allow),
    l("skip", N, Allow),
    l("filter-l3", R, Allow),
    l("filter-tcp", R, Allow),
    l("filter-udp", R, Allow),
    l("filter-l7", R, Allow),
    l("filter-ssid", R, Allow),
    l("hostlist", R, File),
    l("hostlist-exclude", R, File),
    l("hostlist-domains", R, Allow),
    l("hostlist-exclude-domains", R, Allow),
    l("hostlist-auto", R, Deny),
    l("hostlist-auto-debug", R, Deny),
    l("hostlist-auto-fail-threshold", R, Allow),
    l("hostlist-auto-fail-time", R, Allow),
    l("hostlist-auto-retrans-threshold", R, Allow),
    l("ipset", R, File),
    l("ipset-exclude", R, File),
    l("ipset-ip", R, Allow),
    l("ipset-exclude-ip", R, Allow),
    // desync
    l("dpi-desync", R, Allow),
    l("dpi-desync-repeats", R, Allow),
    l("dpi-desync-ttl", R, Allow),
    l("dpi-desync-ttl6", R, Allow),
    l("dpi-desync-autottl", Opt_, Allow),
    l("dpi-desync-autottl6", Opt_, Allow),
    l("dpi-desync-fooling", R, Allow),
    l("dpi-desync-split-pos", R, Allow),
    l("dpi-desync-split-seqovl", R, Allow),
    l("dpi-desync-split-seqovl-pattern", R, PayloadOrFile),
    l("dpi-desync-split-http-req", R, Allow),
    l("dpi-desync-split-tls", R, Allow),
    l("dpi-desync-fakedsplit-pattern", R, PayloadOrFile),
    l("dpi-desync-fakedsplit-mod", R, Allow),
    l("dpi-desync-hostfakesplit-midhost", R, Allow),
    l("dpi-desync-hostfakesplit-mod", R, Allow),
    l("dpi-desync-ipfrag-pos-tcp", R, Allow),
    l("dpi-desync-ipfrag-pos-udp", R, Allow),
    l("dpi-desync-badseq-increment", R, Allow),
    l("dpi-desync-badack-increment", R, Allow),
    l("dpi-desync-ts-increment", R, Allow),
    l("dpi-desync-any-protocol", Opt_, Allow),
    l("dpi-desync-skip-nosni", Opt_, Allow),
    l("dpi-desync-start", R, Allow),
    l("dpi-desync-cutoff", R, Allow),
    l("dpi-desync-fake-tls", R, PayloadOrFile),
    l("dpi-desync-fake-tls-mod", R, Allow),
    l("dpi-desync-fake-tcp-mod", R, Allow),
    l("dpi-desync-fake-http", R, PayloadOrFile),
    l("dpi-desync-fake-quic", R, PayloadOrFile),
    l("dpi-desync-fake-wireguard", R, PayloadOrFile),
    l("dpi-desync-fake-dht", R, PayloadOrFile),
    l("dpi-desync-fake-discord", R, PayloadOrFile),
    l("dpi-desync-fake-stun", R, PayloadOrFile),
    l("dpi-desync-fake-unknown", R, PayloadOrFile),
    l("dpi-desync-fake-unknown-udp", R, PayloadOrFile),
    l("dpi-desync-fake-syndata", R, PayloadOrFile),
    l("dpi-desync-udplen-increment", R, Allow),
    l("dpi-desync-udplen-pattern", R, PayloadOrFile),
    l("dpi-desync-tcp-flags-set", R, Allow),
    l("dpi-desync-tcp-flags-unset", R, Allow),
    // dup / orig
    l("dup", R, Allow),
    l("dup-replace", Opt_, Allow),
    l("dup-ttl", R, Allow),
    l("dup-ttl6", R, Allow),
    l("dup-autottl", Opt_, Allow),
    l("dup-autottl6", Opt_, Allow),
    l("dup-fooling", R, Allow),
    l("dup-ts-increment", R, Allow),
    l("dup-badseq-increment", R, Allow),
    l("dup-badack-increment", R, Allow),
    l("dup-ip-id", R, Allow),
    l("dup-start", R, Allow),
    l("dup-cutoff", R, Allow),
    l("dup-tcp-flags-set", R, Allow),
    l("dup-tcp-flags-unset", R, Allow),
    l("orig-ttl", R, Allow),
    l("orig-ttl6", R, Allow),
    l("orig-autottl", Opt_, Allow),
    l("orig-autottl6", Opt_, Allow),
    l("orig-mod-start", R, Allow),
    l("orig-mod-cutoff", R, Allow),
    l("orig-tcp-flags-set", R, Allow),
    l("orig-tcp-flags-unset", R, Allow),
    // misc
    l("ip-id", R, Allow),
    l("wssize", R, Allow),
    l("wssize-cutoff", R, Allow),
    l("wssize-forced-cutoff", Opt_, Allow),
    l("synack-split", Opt_, Allow),
    l("ctrack-timeouts", R, Allow),
    l("ctrack-disable", Opt_, Allow),
    l("ipcache-lifetime", R, Allow),
    l("ipcache-hostname", Opt_, Allow),
    l("comment", Opt_, Allow),
    l("debug", Opt_, DebugLevel),
    l("dry-run", N, Deny),
    l("version", N, Deny),
    l("daemon", N, Deny),
    l("pidfile", R, Deny),
];

/// tpws (zapret v72, Linux): a transparent/SOCKS proxy. DPIMech runs it as a SOCKS server on
/// 127.0.0.1; everything about where it listens is managed.
const TPWS: &[Opt] = &[
    l("bind-addr", R, Managed),
    l("bind-iface4", R, Managed),
    l("bind-iface6", R, Managed),
    l("bind-linklocal", R, Managed),
    l("bind-wait-ifup", R, Managed),
    l("bind-wait-ip", R, Managed),
    l("bind-wait-ip-linklocal", R, Managed),
    l("bind-wait-only", N, Managed),
    l("port", R, Managed),
    l("socks", N, Managed),
    l("connect-bind-addr", R, Allow),
    l("no-resolve", N, Allow),
    l("resolver-threads", R, Allow),
    l("local-rcvbuf", R, Allow),
    l("local-sndbuf", R, Allow),
    l("remote-rcvbuf", R, Allow),
    l("remote-sndbuf", R, Allow),
    l("nosplice", N, Allow),
    l("local-tcp-user-timeout", R, Allow),
    l("remote-tcp-user-timeout", R, Allow),
    l("maxconn", R, Allow),
    l("fix-seg", Opt_, Allow),
    l("ipcache-lifetime", R, Allow),
    l("ipcache-hostname", Opt_, Allow),
    l("debug", Opt_, DebugLevel),
    l("debug-level", R, Allow),
    l("comment", Opt_, Allow),
    l("new", N, Allow),
    l("skip", N, Allow),
    l("filter-l3", R, Allow),
    l("filter-tcp", R, Allow),
    l("filter-l7", R, Allow),
    l("hostlist", R, File),
    l("hostlist-exclude", R, File),
    l("hostlist-domains", R, Allow),
    l("hostlist-exclude-domains", R, Allow),
    l("hostlist-auto", R, Deny),
    l("hostlist-auto-debug", R, Deny),
    l("hostlist-auto-fail-threshold", R, Allow),
    l("hostlist-auto-fail-time", R, Allow),
    l("ipset", R, File),
    l("ipset-exclude", R, File),
    l("ipset-ip", R, Allow),
    l("ipset-exclude-ip", R, Allow),
    l("split-pos", R, Allow),
    l("split-any-protocol", N, Allow),
    l("disorder", Opt_, Allow),
    l("oob", Opt_, Allow),
    l("oob-data", R, Allow),
    l("tlsrec", R, Allow),
    l("mss", R, Allow),
    l("hostcase", N, Allow),
    l("hostspell", R, Allow),
    l("hostdot", N, Allow),
    l("hosttab", N, Allow),
    l("hostnospace", N, Allow),
    l("hostpad", R, Allow),
    l("domcase", N, Allow),
    l("methodspace", N, Allow),
    l("methodeol", N, Allow),
    l("unixeol", N, Allow),
    l("tamper-start", R, Allow),
    l("tamper-cutoff", R, Allow),
    l("daemon", N, Deny),
    l("pidfile", R, Deny),
    l("user", R, Deny),
    l("uid", R, Deny),
    l("version", N, Deny),
];

/// GoodbyeDPI 0.2.x.
const GOODBYEDPI: &[Opt] = &[
    o('p', "", N, Allow),
    o('q', "", N, Allow),
    o('r', "", N, Allow),
    o('s', "", N, Allow),
    o('m', "", N, Allow),
    o('n', "", N, Allow),
    o('a', "", N, Allow),
    o('w', "", N, Allow),
    o('f', "", R, Allow),
    o('k', "", R, Allow),
    o('e', "", R, Allow),
    o('1', "", N, Allow),
    o('2', "", N, Allow),
    o('3', "", N, Allow),
    o('4', "", N, Allow),
    o('5', "", N, Allow),
    o('6', "", N, Allow),
    o('7', "", N, Allow),
    o('8', "", N, Allow),
    o('9', "", N, Allow),
    l("port", R, Allow),
    l("ip-id", R, Allow),
    l("dns-addr", R, Allow),
    l("dns-port", R, Allow),
    l("dnsv6-addr", R, Allow),
    l("dnsv6-port", R, Allow),
    l("dns-verb", N, Allow),
    l("blacklist", R, File),
    l("allow-no-sni", N, Allow),
    l("frag-by-sni", N, Allow),
    l("set-ttl", R, Allow),
    l("auto-ttl", Opt_, Allow),
    l("min-ttl", R, Allow),
    l("wrong-chksum", N, Allow),
    l("wrong-seq", N, Allow),
    l("native-frag", N, Allow),
    l("reverse-frag", N, Allow),
    l("max-payload", Opt_, Allow),
    l("fake-from-hex", R, Allow),
    l("fake-with-sni", R, Allow),
    l("fake-gen", R, Allow),
    l("fake-resend", R, Allow),
];

/// An engine's options, possibly assembled from shared parts.
type Table = &'static [&'static [Opt]];

fn table(engine: EngineKind) -> Option<Table> {
    match engine {
        EngineKind::ByeDpi => Some(&[BYEDPI]),
        EngineKind::ZapretWinws => Some(&[WINWS_FILTERS, ZAPRET_COMMON]),
        EngineKind::ZapretNfqws => Some(&[NFQWS_ONLY, ZAPRET_COMMON]),
        EngineKind::ZapretTpws => Some(&[TPWS]),
        EngineKind::GoodbyeDpi => Some(&[GOODBYEDPI]),
        // TODO(phase 6): SpoofDPI.
        EngineKind::SpoofDpi => None,
    }
}

fn options(table: Table) -> impl Iterator<Item = &'static Opt> + Clone {
    table.iter().flat_map(|part| part.iter())
}

/// Checks user-supplied engine arguments. Files may only be read from `readable_dirs`
/// (DPIMech's lists directory and the engine's own payload folder).
pub fn check_engine_args(
    engine: EngineKind,
    args: &[String],
    readable_dirs: &[&Path],
) -> Result<(), String> {
    if args.is_empty() {
        return Ok(());
    }
    let table = table(engine).ok_or_else(|| {
        format!(
            "custom arguments for {} are not supported yet",
            engine.display_name()
        )
    })?;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v.to_owned())),
                None => (long, None),
            };
            let opt = find_long(table, name)?;
            let value = match (opt.arg, inline) {
                (Arg::None, Some(_)) => return Err(format!("{arg}: option takes no value")),
                (Arg::None, None) | (Arg::Optional, None) => None,
                (_, Some(v)) => Some(v),
                (Arg::Required, None) => Some(next_value(args, &mut i, arg)?),
            };
            check(opt, value.as_deref(), readable_dirs)?;
        } else if let Some(cluster) = arg.strip_prefix('-').filter(|c| !c.is_empty()) {
            // getopt: flags may be clustered; an option with a value consumes the rest of the
            // cluster (or, if required and nothing is left, the next argument).
            for (pos, c) in cluster.char_indices() {
                let opt = options(table)
                    .find(|o| o.short == Some(c))
                    .ok_or_else(|| format!("unknown option -{c}"))?;
                if opt.arg == Arg::None {
                    check(opt, None, readable_dirs)?;
                    continue;
                }
                let rest = &cluster[pos + c.len_utf8()..];
                let value = match (rest.is_empty(), opt.arg) {
                    (false, _) => Some(rest.to_owned()),
                    (true, Arg::Required) => Some(next_value(args, &mut i, arg)?),
                    (true, _) => None,
                };
                check(opt, value.as_deref(), readable_dirs)?;
                break;
            }
        } else {
            return Err(format!("unexpected argument \"{arg}\""));
        }
    }
    Ok(())
}

/// getopt_long accepts any unambiguous prefix of a long option.
fn find_long(table: Table, name: &str) -> Result<&'static Opt, String> {
    let named = || options(table).filter(|o| !o.long.is_empty());
    if let Some(exact) = named().find(|o| o.long == name) {
        return Ok(exact);
    }
    let mut matches = named().filter(|o| o.long.starts_with(name));
    match (matches.next(), matches.next()) {
        (Some(o), None) if !name.is_empty() => Ok(o),
        (Some(_), Some(_)) => Err(format!("ambiguous option --{name}")),
        _ => Err(format!("unknown option --{name}")),
    }
}

fn next_value(args: &[String], i: &mut usize, opt: &str) -> Result<String, String> {
    let value = args
        .get(*i)
        .cloned()
        .ok_or_else(|| format!("{opt} needs a value"))?;
    *i += 1;
    Ok(value)
}

fn check(opt: &Opt, value: Option<&str>, dirs: &[&Path]) -> Result<(), String> {
    let name = if opt.long.is_empty() {
        format!("-{}", opt.short.unwrap_or('?'))
    } else {
        format!("--{}", opt.long)
    };
    let value = value.unwrap_or_default();
    let file_ok = |path: &str| dirs.iter().any(|d| is_inside(Path::new(path), d));
    let where_ = || {
        dirs.iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(" or ")
    };
    match opt.policy {
        Allow => Ok(()),
        Managed => Err(format!(
            "{name} is set by DPIMech (listen address, port or packet queue); remove it — the port is chosen in the profile"
        )),
        Deny => Err(format!("{name} is not allowed (writes files or exits)")),
        StdoutOnly if value == "-" => Ok(()),
        StdoutOnly => Err(format!("{name} may only write to \"-\" (log)")),
        DebugLevel if value.is_empty() || value.chars().all(|c| c.is_ascii_digit()) => Ok(()),
        DebugLevel => Err(format!("{name} only accepts a level (0, 1, 2)")),
        InlineOrFile if value.starts_with(':') || file_ok(value) => Ok(()),
        InlineOrFile => Err(format!(
            "{name} accepts \":inline text\" or a file in {}",
            where_()
        )),
        File if file_ok(value) => Ok(()),
        File => Err(format!("{name} must point to a file in {}", where_())),
        PayloadOrFile => match value.split_once('@') {
            // `[+ofs]@path`
            Some((ofs, path)) if ofs.is_empty() || ofs.starts_with('+') => {
                if file_ok(path) {
                    Ok(())
                } else {
                    Err(format!("{name} must use a file in {}", where_()))
                }
            }
            Some(_) => Err(format!("{name}: unexpected value \"{value}\"")),
            None if value.starts_with("0x") || value.starts_with('!') || value.is_empty() => Ok(()),
            // winws also treats a bare token as a file name.
            None if file_ok(value) => Ok(()),
            None => Err(format!("{name} accepts 0xHEX or @file in {}", where_())),
        },
    }
}

/// Existing paths are compared after resolving symlinks and `..`. A path that does not exist
/// yet (engine not installed when the profile is saved) is accepted only if it is absolute,
/// has no `..` and sits lexically under `dir`; the launch-time check sees the real file.
fn is_inside(path: &Path, dir: &Path) -> bool {
    use std::path::Component;
    if let Ok(p) = path.canonicalize() {
        return dir.canonicalize().is_ok_and(|d| p.starts_with(d));
    }
    path.is_absolute()
        && !path.components().any(|c| matches!(c, Component::ParentDir))
        && (path.starts_with(dir) || dir.canonicalize().is_ok_and(|d| path.starts_with(d)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::split_args;

    fn lists() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("dpimech-argpolicy-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hosts.txt"), "discord.com").unwrap();
        std::fs::write(dir.join("fake.bin"), [0u8; 4]).unwrap();
        dir
    }

    fn check(engine: EngineKind, s: &str) -> Result<(), String> {
        let dir = lists();
        let s = s.replace("$LISTS", &dir.display().to_string());
        check_engine_args(engine, &split_args(&s), &[&dir])
    }

    fn bye(s: &str) -> Result<(), String> {
        check(EngineKind::ByeDpi, s)
    }

    fn winws(s: &str) -> Result<(), String> {
        check(EngineKind::ZapretWinws, s)
    }

    fn nfqws(s: &str) -> Result<(), String> {
        check(EngineKind::ZapretNfqws, s)
    }

    fn tpws(s: &str) -> Result<(), String> {
        check(EngineKind::ZapretTpws, s)
    }

    fn gdpi(s: &str) -> Result<(), String> {
        check(EngineKind::GoodbyeDpi, s)
    }

    #[test]
    fn byedpi_allows_normal_strategies() {
        assert!(bye("-r 1+s").is_ok());
        assert!(bye("-o1 -a1 -An -Ku -a1 -An -s1 -d3+s -At,r,s -s1 -o2").is_ok());
        assert!(bye("--split 1+s --disorder=3 -NU -c 2048").is_ok());
        assert!(bye("--tlsr 1+s").is_ok(), "unique long prefix");
        assert!(bye("-H :discord.com -l :GET").is_ok());
        assert!(bye(r#"-H "$LISTS/hosts.txt""#).is_ok());
        assert!(bye("-y -").is_ok());
    }

    #[test]
    fn byedpi_rejects_dangerous_or_managed_options() {
        assert!(bye("-y C:/Windows/evil.dll").is_err());
        assert!(bye("--cache-d=C:/x").is_err(), "prefix of --cache-dump");
        assert!(bye("-NUyC:/x").is_err(), "clustered short options");
        assert!(bye("-l C:/Windows/System32/config/SAM").is_err());
        assert!(bye(r#"-H "$LISTS/../../secret.txt""#).is_err());
        assert!(bye("-i 0.0.0.0").is_err());
        assert!(bye("--port 9999").is_err());
        assert!(bye("--daemon").is_err(), "unknown options are rejected");
        assert!(bye("stray").is_err());
        assert!(bye("-r").is_err(), "missing value");
    }

    #[test]
    fn winws_allows_typical_strategies() {
        assert!(winws(
            r#"--wf-tcp=80,443 --wf-udp=443 --filter-udp=443 --hostlist="$LISTS/hosts.txt" --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic="$LISTS/fake.bin" --new --filter-tcp=80,443 --hostlist="$LISTS/hosts.txt" --dpi-desync=multisplit --dpi-desync-split-seqovl=681 --dpi-desync-split-pos=1 --dpi-desync-split-seqovl-pattern="$LISTS/fake.bin""#
        )
        .is_ok());
        assert!(winws("--dpi-desync=fake,multidisorder --dpi-desync-fooling=badseq --dpi-desync-fake-tls=0x00000000 --dpi-desync-autottl").is_ok());
        assert!(
            winws(r#"--dpi-desync-fake-tls=+10@$LISTS/fake.bin --dpi-desync-fake-tls=! --debug=1"#)
                .is_ok()
        );
        assert!(winws("--dpi-desync split").is_ok(), "value as next token");
    }

    #[test]
    fn winws_rejects_file_writes_and_outside_reads() {
        assert!(winws("--debug=@C:/Windows/System32/evil.txt").is_err());
        assert!(winws("--hostlist-auto=C:/x.txt").is_err());
        assert!(winws("--wf-save=C:/x.txt").is_err());
        assert!(winws("--pidfile=C:/x").is_err());
        assert!(winws("--hostlist=C:/Windows/win.ini").is_err());
        assert!(winws("--dpi-desync-fake-tls=@C:/Windows/win.ini").is_err());
        assert!(
            winws("@C:/config.txt").is_err(),
            "config files are not allowed"
        );
        assert!(winws("--no-such-option").is_err());
    }

    #[test]
    fn goodbyedpi_modes_and_blacklist() {
        assert!(gdpi("-9").is_ok());
        assert!(gdpi("-9 --dns-addr 77.88.8.8 --dns-port 1253 --dnsv6-addr 2a02:6b8::feed:0ff --dnsv6-port 1253").is_ok());
        assert!(gdpi("-e2 --frag-by-sni --set-ttl 5 --auto-ttl=1-4-10").is_ok());
        assert!(gdpi(r#"--blacklist "$LISTS/hosts.txt" -5"#).is_ok());
        assert!(gdpi("--blacklist C:/Windows/win.ini").is_err());
        assert!(gdpi("-x").is_err());
    }

    #[test]
    fn nfqws_shares_the_desync_options_but_not_the_windivert_filters() {
        assert!(nfqws(r#"--filter-tcp=80,443 --hostlist="$LISTS/hosts.txt" --dpi-desync=fake,multidisorder --dpi-desync-split-pos=1,midsld --dpi-desync-fooling=badseq --new --filter-udp=443 --dpi-desync=fake --dpi-desync-fake-quic="$LISTS/fake.bin""#).is_ok());
        assert!(nfqws("--wf-tcp=80,443").is_err());
        assert!(nfqws("--qnum=5").is_err(), "queue is managed");
        assert!(nfqws("--dpi-desync-fwmark=0x1").is_err(), "mark is managed");
        assert!(nfqws("--user=nobody").is_err());
        assert!(nfqws("--debug=@/etc/cron.d/x").is_err());
        assert!(nfqws("--pidfile=/tmp/x").is_err());
        assert!(nfqws("--hostlist=/etc/shadow").is_err());
    }

    #[test]
    fn tpws_strategies_but_no_listen_or_file_writes() {
        assert!(tpws(r#"--hostlist="$LISTS/hosts.txt" --split-pos=1,midsld --disorder --oob --tlsrec=sniext"#).is_ok());
        assert!(tpws("--hostcase --methodeol --debug=1").is_ok());
        assert!(tpws("--port=9999").is_err());
        assert!(tpws("--bind-addr=0.0.0.0").is_err());
        assert!(tpws("--daemon").is_err());
        assert!(tpws("--debug=@/root/.bashrc").is_err());
        assert!(tpws("--hostlist-auto=/etc/x").is_err());
    }
}
