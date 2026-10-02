//! Built-in domain packs and strategy packs.
//!
//! Strategy arguments may use placeholders that the service resolves at launch:
//! - `{hostlist}` – a hosts file generated from the profile's (or the test's) domains
//! - `{fake}`     – zapret's bundled fake-payload folder
//! - `{lists}`    – DPIMech's lists folder
//! - `{sni}`      – a harmless SNI for ByeDPI fake packets

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::EngineKind;

/// A ready-made site pack, from `strategies/packs.json` (shared with the Android app).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DomainPack {
    pub id: String,
    pub name: String,
    /// Everything the engine should act on (goes into the hostlist).
    pub domains: Vec<String>,
    /// Hosts the Strategy Lab requests. Each answers HTTPS on `/`; apex domains without an
    /// A record (e.g. `discordapp.net`) would fail regardless of the strategy and make every
    /// result look worse.
    pub probes: Vec<String>,
    // `android_packages` (Android apps that use these sites) is only read by the Android app.
}

/// The site packs, from `strategies/packs.json` in the repository. Discord stays first: the
/// wizard and the Lab preselect it.
pub const EMBEDDED_PACKS: &str = include_str!("../../../strategies/packs.json");
/// The pack file format this build understands.
const PACK_FORMAT: u32 = 1;

#[derive(Deserialize)]
struct PackFile {
    format: u32,
    packs: Vec<DomainPack>,
}

/// Parses and checks a pack file: unique ids, plain host names, at least one probe each.
pub fn parse_packs(text: &str) -> anyhow::Result<Vec<DomainPack>> {
    let file: PackFile = serde_json::from_str(text)?;
    if file.format != PACK_FORMAT {
        anyhow::bail!("unsupported pack file format {}", file.format);
    }
    if file.packs.is_empty() {
        anyhow::bail!("the pack file has no packs");
    }
    let mut ids = std::collections::HashSet::new();
    for p in &file.packs {
        if p.id.is_empty() || p.name.trim().is_empty() || !ids.insert(p.id.as_str()) {
            anyhow::bail!("pack \"{}\": missing or repeated id or name", p.id);
        }
        if p.domains.is_empty() || p.probes.is_empty() {
            anyhow::bail!("pack \"{}\": no domains or no probes", p.id);
        }
        if let Some(bad) = p.domains.iter().chain(&p.probes).find(|h| !is_host(h)) {
            anyhow::bail!("pack \"{}\": \"{bad}\" is not a host name", p.id);
        }
    }
    Ok(file.packs)
}

/// A lower-case DNS name with at least one dot: what a hosts file and a probe accept.
fn is_host(h: &str) -> bool {
    h.len() <= 253
        && h.contains('.')
        && h.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        })
}

/// The packs built into this binary.
pub fn domain_packs() -> &'static [DomainPack] {
    static PACKS: OnceLock<Vec<DomainPack>> = OnceLock::new();
    PACKS.get_or_init(|| parse_packs(EMBEDDED_PACKS).expect("valid packs.json"))
}

pub fn domain_pack(id: &str) -> Option<&'static DomainPack> {
    domain_packs().iter().find(|p| p.id == id)
}

/// Where DPIMech itself is released (update notifications).
pub const APP_REPO: &str = "halilkhrmn/dpimech";

/// Where problem reports go when the user has no GitHub account.
pub const SUPPORT_EMAIL: &str = "halilkahraman@yandex.com";

/// Label shown for strategies that ship inside DPIMech.
pub const STANDARD_SET: &str = "DPIMech standard set";

/// The standard strategies, from `strategies/default.json` in the repository. The service also
/// fetches that file from `main` ([`STRATEGIES_URL`]), so a change there reaches every user
/// without a release; this copy is the fallback when it cannot.
pub const EMBEDDED_STRATEGIES: &str = include_str!("../../../strategies/default.json");
pub const STRATEGIES_URL: &str =
    "https://raw.githubusercontent.com/halilkhrmn/dpimech/main/strategies/default.json";
/// The file format this build understands; a newer one is ignored until the app is updated.
const STRATEGY_FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StrategyEntry {
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyFile {
    format: u32,
    /// Keyed by the engine's config name (`bye_dpi`, …). Unknown engines are ignored, so a newer
    /// file with an extra engine still works in older builds.
    engines: HashMap<String, Vec<StrategyEntry>>,
}

impl StrategyFile {
    /// Parses and sanity-checks a strategy file. Arguments are not trusted here: the argument
    /// policy checks them again at every launch.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let file: StrategyFile = serde_json::from_str(text)?;
        if file.format != STRATEGY_FORMAT {
            anyhow::bail!("unsupported strategy file format {}", file.format);
        }
        let mut count = 0;
        for (engine, list) in &file.engines {
            for s in list {
                if s.name.trim().is_empty() || s.args.trim().is_empty() {
                    anyhow::bail!("{engine}: a strategy without name or arguments");
                }
                if s.name.len() > 200 || s.args.len() > 4000 {
                    anyhow::bail!("{engine}: strategy \"{}\" is too long", s.name);
                }
            }
            count += list.len();
        }
        if count == 0 {
            anyhow::bail!("the strategy file has no strategies");
        }
        Ok(file)
    }

    /// The copy built into this binary.
    pub fn embedded() -> &'static StrategyFile {
        static FILE: OnceLock<StrategyFile> = OnceLock::new();
        FILE.get_or_init(|| StrategyFile::parse(EMBEDDED_STRATEGIES).expect("valid default.json"))
    }

    pub fn for_engine(&self, engine: EngineKind) -> &[StrategyEntry] {
        // nfqws gets the winws set; `adapt_args` drops the WinDivert filters.
        let key = match engine {
            EngineKind::ByeDpi => "bye_dpi",
            EngineKind::ZapretWinws | EngineKind::ZapretNfqws => "zapret_winws",
            EngineKind::ZapretTpws => "zapret_tpws",
            EngineKind::GoodbyeDpi => "goodbye_dpi",
            EngineKind::SpoofDpi => "spoof_dpi",
        };
        self.engines.get(key).map(Vec::as_slice).unwrap_or_default()
    }
}

pub fn builtin_strategies(engine: EngineKind) -> &'static [StrategyEntry] {
    StrategyFile::embedded().for_engine(engine)
}

/// Turns a strategy written for winws into one for `engine`. On Linux nftables decides what
/// reaches nfqws (see the service's `nfqueue`), so the `--wf-*` WinDivert filters go.
pub fn adapt_args(engine: EngineKind, args: &str) -> String {
    if engine != EngineKind::ZapretNfqws {
        return args.to_owned();
    }
    let tokens = crate::args::split_args(args);
    let mut out: Vec<String> = Vec::new();
    let mut iter = tokens.into_iter();
    while let Some(token) = iter.next() {
        if token.starts_with("--wf-") {
            if !token.contains('=') {
                iter.next();
            }
            continue;
        }
        out.push(if token.contains(' ') {
            format!("\"{token}\"")
        } else {
            token
        });
    }
    out.join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// One argument line per line; `#` starts a comment.
    ArgsPerLine,
    /// `Name:args` per line (SplitWire-Turkey presets).
    NameColonArgs,
}

/// Frequently updated community strategy lists, fetched at runtime (never bundled).
pub struct OnlineSource {
    /// What the user sees next to each strategy, e.g. "Community list".
    pub label: &'static str,
    /// Where the list comes from (credited in the UI).
    pub origin: &'static str,
    pub url: &'static str,
    pub format: SourceFormat,
    /// ISO country the presets were made for, if any.
    pub country: Option<&'static str>,
}

pub fn online_sources(engine: EngineKind) -> &'static [OnlineSource] {
    match engine {
        EngineKind::ByeDpi => &[OnlineSource {
            label: "Community list",
            origin: "github.com/romanvht/ByeDPIManager",
            url: "https://raw.githubusercontent.com/romanvht/ByeDPIManager/main/proxytest/strategies.txt",
            format: SourceFormat::ArgsPerLine,
            country: None,
        }],
        EngineKind::ZapretWinws => &[OnlineSource {
            label: "Turkey ISP presets",
            origin: "github.com/cagritaskn/SplitWire-Turkey",
            url: "https://raw.githubusercontent.com/cagritaskn/SplitWire-Turkey/main/src/SplitWireTurkey/Resources/zapret/zapret-winws/presets.txt",
            format: SourceFormat::NameColonArgs,
            country: Some("TR"),
        }],
        EngineKind::GoodbyeDpi => &[OnlineSource {
            label: "Turkey ISP presets",
            origin: "github.com/cagritaskn/SplitWire-Turkey",
            url: "https://raw.githubusercontent.com/cagritaskn/SplitWire-Turkey/main/src/SplitWireTurkey/Resources/goodbyedpi/presets.txt",
            format: SourceFormat::NameColonArgs,
            country: Some("TR"),
        }],
        _ => &[],
    }
}

/// Parses an online list into `(name, args)` pairs.
pub fn parse_source(format: SourceFormat, text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match format {
            SourceFormat::ArgsPerLine => out.push((format!("#{}", i + 1), line.to_owned())),
            SourceFormat::NameColonArgs => {
                // Names never start with '-'; args always do.
                if let Some((name, args)) = line.split_once(':')
                    && !name.trim().starts_with('-')
                {
                    out.push((name.trim().to_owned(), args.trim().to_owned()));
                }
            }
        }
    }
    out
}

/// Well-known ISPs, matched on the ASN or the provider name reported by the lookup service.
/// `keys` are matched (case-insensitively) against preset names such as "Türk Telekom".
pub struct Isp {
    pub name: &'static str,
    pub asns: &'static [u32],
    pub name_hints: &'static [&'static str],
    pub preset_keys: &'static [&'static str],
}

pub const ISPS: &[Isp] = &[
    Isp {
        name: "Türk Telekom",
        asns: &[9121],
        name_hints: &["turk telekom", "türk telekom", "ttnet"],
        preset_keys: &["türk telekom", "turk telekom"],
    },
    Isp {
        name: "Superonline (Turkcell)",
        asns: &[34984],
        name_hints: &["superonline"],
        preset_keys: &["superonline"],
    },
    Isp {
        name: "Turkcell (mobile)",
        asns: &[16135],
        name_hints: &["turkcell"],
        preset_keys: &["turkcell"],
    },
    Isp {
        name: "Vodafone Türkiye",
        asns: &[15897],
        name_hints: &["vodafone"],
        preset_keys: &["vodafone"],
    },
    Isp {
        name: "TurkNet",
        asns: &[12735],
        name_hints: &["turknet", "turk net"],
        preset_keys: &["turknet"],
    },
    Isp {
        name: "Kablonet (Türksat)",
        asns: &[47524],
        name_hints: &["turksat", "türksat", "kablonet"],
        preset_keys: &["kablonet"],
    },
];

pub fn match_isp(asn: Option<u32>, provider: &str) -> Option<&'static Isp> {
    let provider = provider.to_lowercase();
    ISPS.iter().find(|isp| {
        asn.is_some_and(|a| isp.asns.contains(&a))
            || isp.name_hints.iter().any(|h| provider.contains(h))
    })
}

/// Winws / GoodbyeDPI presets from other sources often act on all traffic. For tests and
/// for profiles created from them, scope them to the chosen sites.
pub fn scope_to_hostlist(engine: EngineKind, args: &str) -> String {
    match engine {
        EngineKind::ZapretWinws | EngineKind::ZapretNfqws
            if !args.contains("--hostlist")
                && !args.contains("--ipset")
                && !args.contains("--new") =>
        {
            format!("{args} --hostlist={{hostlist}}")
        }
        EngineKind::GoodbyeDpi if !args.contains("--blacklist") => {
            format!("{args} --blacklist {{hostlist}}")
        }
        _ => args.to_owned(),
    }
}

pub const DEFAULT_SNI: &str = "www.google.com";

/// Whether the strategy needs a `{hostlist}` of domains to be meaningful.
pub fn uses_hostlist(args: &str) -> bool {
    args.contains("{hostlist}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_packs_parse_with_discord_first() {
        let packs = domain_packs();
        assert_eq!(packs[0].id, "discord");
        assert!(domain_pack("youtube").is_some());
        assert!(packs.len() >= 6);
    }

    #[test]
    fn pack_file_rejects_bad_hosts_and_formats() {
        let ok = r#"{"format":1,"packs":[{"id":"a","name":"A","domains":["a.com"],"probes":["a.com"]}]}"#;
        assert_eq!(parse_packs(ok).unwrap()[0].domains, ["a.com"]);
        for bad in [
            ok.replace("\"format\":1", "\"format\":2"),
            ok.replace("[\"a.com\"],\"probes", "[\"a.com/x\"],\"probes"),
            ok.replace("[\"a.com\"],\"probes", "[\"-a.com\"],\"probes"),
            ok.replace("\"probes\":[\"a.com\"]", "\"probes\":[]"),
            ok.replace(
                "}]}",
                "},{\"id\":\"a\",\"name\":\"B\",\"domains\":[\"b.com\"],\"probes\":[\"b.com\"]}]}",
            ),
        ] {
            assert!(parse_packs(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn nfqws_strategies_lose_only_the_windivert_filters() {
        for s in builtin_strategies(EngineKind::ZapretNfqws) {
            let args = adapt_args(EngineKind::ZapretNfqws, &s.args);
            assert!(!args.contains("--wf-"), "{}", s.name);
            assert!(args.contains("--dpi-desync"), "{}", s.name);
        }
        assert_eq!(
            adapt_args(
                EngineKind::ZapretNfqws,
                "--wf-tcp 80 --wf-udp=443 --filter-tcp=80 --dpi-desync=fake"
            ),
            "--filter-tcp=80 --dpi-desync=fake"
        );
        assert_eq!(
            adapt_args(EngineKind::ZapretWinws, "--wf-tcp=80"),
            "--wf-tcp=80"
        );
    }

    #[test]
    fn proxy_strategies_pass_the_argument_policy() {
        let dir = std::env::temp_dir();
        for engine in [
            EngineKind::ByeDpi,
            EngineKind::ZapretTpws,
            EngineKind::SpoofDpi,
        ] {
            for s in builtin_strategies(engine) {
                let args = crate::args::split_args(&s.args);
                assert!(
                    crate::argpolicy::check_engine_args(engine, &args, &[&dir]).is_ok(),
                    "{}: {}",
                    engine.display_name(),
                    s.name
                );
            }
        }
    }

    #[test]
    fn strategy_file_covers_every_engine_and_rejects_bad_files() {
        for engine in [
            EngineKind::ByeDpi,
            EngineKind::ZapretWinws,
            EngineKind::ZapretNfqws,
            EngineKind::ZapretTpws,
            EngineKind::GoodbyeDpi,
            EngineKind::SpoofDpi,
        ] {
            assert!(
                !builtin_strategies(engine).is_empty(),
                "{}",
                engine.display_name()
            );
        }
        let newer = r#"{"format": 2, "engines": {"bye_dpi": [{"name": "a", "args": "-s1"}]}}"#;
        assert!(StrategyFile::parse(newer).is_err());
        assert!(StrategyFile::parse(r#"{"format": 1, "engines": {}}"#).is_err());
        let blank = r#"{"format": 1, "engines": {"bye_dpi": [{"name": "a", "args": " "}]}}"#;
        assert!(StrategyFile::parse(blank).is_err());
        // An engine this build does not know is skipped, not an error.
        let extra = r#"{"format": 1, "engines": {"future": [{"name": "a", "args": "-x"}],
            "bye_dpi": [{"name": "b", "args": "-s1"}]}}"#;
        let file = StrategyFile::parse(extra).unwrap();
        assert_eq!(file.for_engine(EngineKind::ByeDpi).len(), 1);
    }
}
