//! Built-in domain packs and strategy packs.
//!
//! Strategy arguments may use placeholders that the service resolves at launch:
//! - `{hostlist}` – a hosts file generated from the profile's (or the test's) domains
//! - `{fake}`     – zapret's bundled fake-payload folder
//! - `{lists}`    – DPIMech's lists folder
//! - `{sni}`      – a harmless SNI for ByeDPI fake packets

use crate::model::EngineKind;

pub struct DomainPack {
    pub id: &'static str,
    pub name: &'static str,
    /// Everything the engine should act on (goes into the hostlist).
    pub domains: &'static [&'static str],
    /// Hosts the Strategy Lab requests. Each answers HTTPS on `/` (checked 2026-09-30);
    /// apex domains without an A record (e.g. `discordapp.net`) would fail regardless of the
    /// strategy and make every result look worse.
    pub probes: &'static [&'static str],
}

pub const DOMAIN_PACKS: &[DomainPack] = &[
    DomainPack {
        id: "discord",
        name: "Discord",
        domains: &[
            "discord.com",
            "discordapp.com",
            "discord.gg",
            "discord.media",
            "discordapp.net",
            "gateway.discord.gg",
            "cdn.discordapp.com",
            "media.discordapp.net",
            "images-ext-1.discordapp.net",
            "updates.discord.com",
            "dis.gd",
        ],
        probes: &[
            "discord.com",
            "discordapp.com",
            "discord.gg",
            "gateway.discord.gg",
            "cdn.discordapp.com",
            "media.discordapp.net",
            "updates.discord.com",
            "dis.gd",
        ],
    },
    DomainPack {
        id: "youtube",
        name: "YouTube",
        domains: &[
            "youtube.com",
            "www.youtube.com",
            "youtu.be",
            "i.ytimg.com",
            "yt3.ggpht.com",
            "youtubei.googleapis.com",
            "manifest.googlevideo.com",
            "redirector.googlevideo.com",
            "googlevideo.com",
        ],
        probes: &[
            "www.youtube.com",
            "youtube.com",
            "youtu.be",
            "i.ytimg.com",
            "yt3.ggpht.com",
            "youtubei.googleapis.com",
            "redirector.googlevideo.com",
        ],
    },
    DomainPack {
        id: "roblox",
        name: "Roblox",
        domains: &[
            "roblox.com",
            "www.roblox.com",
            "rbxcdn.com",
            "apis.roblox.com",
        ],
        probes: &["www.roblox.com", "roblox.com", "apis.roblox.com"],
    },
    DomainPack {
        id: "x",
        name: "X / Twitter",
        domains: &["x.com", "twitter.com", "twimg.com", "pbs.twimg.com"],
        probes: &["x.com", "twitter.com", "pbs.twimg.com"],
    },
    DomainPack {
        id: "instagram",
        name: "Instagram",
        domains: &["instagram.com", "www.instagram.com", "cdninstagram.com"],
        probes: &["www.instagram.com", "instagram.com"],
    },
    DomainPack {
        id: "wattpad",
        name: "Wattpad",
        domains: &["wattpad.com", "www.wattpad.com"],
        probes: &["www.wattpad.com", "wattpad.com"],
    },
];

pub fn domain_pack(id: &str) -> Option<&'static DomainPack> {
    DOMAIN_PACKS.iter().find(|p| p.id == id)
}

/// Where DPIMech itself is released (update notifications).
pub const APP_REPO: &str = "halilkhrmn/dpimech";

/// Where problem reports go when the user has no GitHub account.
pub const SUPPORT_EMAIL: &str = "halilkahraman@yandex.com";

/// Label shown for strategies that ship inside DPIMech.
pub const STANDARD_SET: &str = "DPIMech standard set";

pub struct BuiltinStrategy {
    pub name: &'static str,
    pub args: &'static str,
}

/// Well-known ByeDPI strategies (public forum/README examples). Larger, frequently updated
/// lists are fetched at runtime by the Strategy Lab instead of being bundled.
const BYEDPI: &[BuiltinStrategy] = &[
    BuiltinStrategy {
        name: "TLS record split",
        args: "-r 1+s",
    },
    BuiltinStrategy {
        name: "Disorder SNI",
        args: "-d1 -s1+s",
    },
    BuiltinStrategy {
        name: "OOB + disorder",
        args: "-o1 -d1",
    },
    BuiltinStrategy {
        name: "Split + TLS record",
        args: "-s1 -r1+s",
    },
    BuiltinStrategy {
        name: "Split SNI + OOB",
        args: "-s1+s -o1+s",
    },
    BuiltinStrategy {
        name: "Fake + TTL",
        args: "-f1 -t6 -n {sni}",
    },
    BuiltinStrategy {
        name: "Fake modified CH",
        args: "-f-1 -Qr -n {sni} -t8",
    },
    BuiltinStrategy {
        name: "Disorder multi",
        args: "-d1 -d3+s -s6+s -d9+s -s12+s",
    },
    BuiltinStrategy {
        name: "Auto: disorder → split",
        args: "-d1 -a1 -At,r,s -s1+s -r1+s",
    },
    BuiltinStrategy {
        name: "Auto: TLS rec → fake",
        args: "-r1+s -a1 -At,r,s -f-1 -n {sni} -t6",
    },
    BuiltinStrategy {
        name: "HTTP host mixcase + split",
        args: "-Mh,d,r -s1 -r1+s",
    },
    BuiltinStrategy {
        name: "OOB SNI + TLS rec",
        args: "-q1+s -r1+s",
    },
];

/// winws strategies adapted from the zapret docs and popular community presets.
/// Every one is scoped to `{hostlist}` so only the chosen domains are touched.
const WINWS: &[BuiltinStrategy] = &[
    BuiltinStrategy {
        name: "multisplit seqovl (Google CH) + QUIC fake",
        args: "--wf-tcp=80,443 --wf-udp=443 --filter-udp=443 --hostlist={hostlist} --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic={fake}/quic_initial_www_google_com.bin --new --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=multisplit --dpi-desync-split-seqovl=681 --dpi-desync-split-pos=1 --dpi-desync-split-seqovl-pattern={fake}/tls_clienthello_www_google_com.bin",
    },
    BuiltinStrategy {
        name: "fake + multidisorder (badseq)",
        args: "--wf-tcp=80,443 --wf-udp=443 --filter-udp=443 --hostlist={hostlist} --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic={fake}/quic_initial_www_google_com.bin --new --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=fake,multidisorder --dpi-desync-split-pos=1,midsld --dpi-desync-repeats=8 --dpi-desync-fooling=badseq --dpi-desync-fake-tls={fake}/tls_clienthello_www_google_com.bin",
    },
    BuiltinStrategy {
        name: "fake + multisplit (md5sig)",
        args: "--wf-tcp=80,443 --wf-udp=443 --filter-udp=443 --hostlist={hostlist} --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic={fake}/quic_initial_www_google_com.bin --new --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=fake,multisplit --dpi-desync-split-pos=1 --dpi-desync-fooling=md5sig --dpi-desync-fake-tls={fake}/tls_clienthello_www_google_com.bin",
    },
    BuiltinStrategy {
        name: "fakedsplit (autottl)",
        args: "--wf-tcp=80,443 --wf-udp=443 --filter-udp=443 --hostlist={hostlist} --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic={fake}/quic_initial_www_google_com.bin --new --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=fake,fakedsplit --dpi-desync-split-pos=1 --dpi-desync-autottl --dpi-desync-fooling=badseq --dpi-desync-repeats=8",
    },
    BuiltinStrategy {
        name: "multisplit sniext",
        args: "--wf-tcp=80,443 --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=multisplit --dpi-desync-split-pos=1,sniext+1",
    },
    BuiltinStrategy {
        name: "multidisorder midsld",
        args: "--wf-tcp=80,443 --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=multidisorder --dpi-desync-split-pos=1,midsld",
    },
    BuiltinStrategy {
        name: "fake (ttl 4) + split",
        args: "--wf-tcp=80,443 --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=fake,split2 --dpi-desync-ttl=4 --dpi-desync-fake-tls={fake}/tls_clienthello_iana_org.bin",
    },
    BuiltinStrategy {
        name: "hostfakesplit",
        args: "--wf-tcp=80,443 --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=hostfakesplit --dpi-desync-fooling=badseq --dpi-desync-repeats=4",
    },
    BuiltinStrategy {
        name: "Discord voice (UDP) + TLS multisplit",
        args: "--wf-tcp=80,443 --wf-udp=443,50000-50100 --filter-udp=50000-50100 --filter-l7=discord,stun --dpi-desync=fake --dpi-desync-repeats=6 --new --filter-udp=443 --hostlist={hostlist} --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-quic={fake}/quic_initial_www_google_com.bin --new --filter-tcp=80,443 --hostlist={hostlist} --dpi-desync=multisplit --dpi-desync-split-seqovl=681 --dpi-desync-split-pos=1 --dpi-desync-split-seqovl-pattern={fake}/tls_clienthello_www_google_com.bin",
    },
];

/// tpws strategies (zapret docs). tpws works on the TCP stream, so these are split, disorder
/// and TLS record tricks; it has no fake packets.
const TPWS: &[BuiltinStrategy] = &[
    BuiltinStrategy {
        name: "split SNI (midsld) + disorder",
        args: "--split-pos=1,midsld --disorder",
    },
    BuiltinStrategy {
        name: "TLS record split at SNI",
        args: "--tlsrec=sniext",
    },
    BuiltinStrategy {
        name: "TLS record + split",
        args: "--tlsrec=sniext --split-pos=1,midsld",
    },
    BuiltinStrategy {
        name: "split + OOB",
        args: "--split-pos=1 --oob",
    },
    BuiltinStrategy {
        name: "disorder only",
        args: "--split-pos=2 --disorder",
    },
    BuiltinStrategy {
        name: "HTTP host case + split",
        args: "--hostcase --split-pos=method+2,midsld",
    },
    BuiltinStrategy {
        name: "small MSS (split by the network)",
        args: "--mss=88",
    },
    BuiltinStrategy {
        name: "TLS record + disorder + OOB",
        args: "--tlsrec=sniext --split-pos=1,midsld --disorder --oob",
    },
];

/// GoodbyeDPI presets. `-9` is the modern default; the DNS variant also defeats DNS spoofing.
const GOODBYEDPI: &[BuiltinStrategy] = &[
    BuiltinStrategy {
        name: "-9 (recommended)",
        args: "-9 --blacklist {hostlist}",
    },
    BuiltinStrategy {
        name: "-9 + DNS redirect",
        args: "-9 --blacklist {hostlist} --dns-addr 77.88.8.8 --dns-port 1253 --dnsv6-addr 2a02:6b8::feed:0ff --dnsv6-port 1253",
    },
    BuiltinStrategy {
        name: "-5 (auto TTL)",
        args: "-5 --blacklist {hostlist}",
    },
    BuiltinStrategy {
        name: "-6 (wrong seq)",
        args: "-6 --blacklist {hostlist}",
    },
    BuiltinStrategy {
        name: "-7 (wrong checksum)",
        args: "-7 --blacklist {hostlist}",
    },
    BuiltinStrategy {
        name: "fragment by SNI + TTL 5",
        args: "-e2 --frag-by-sni --set-ttl 5 --blacklist {hostlist}",
    },
    BuiltinStrategy {
        name: "native frag + wrong seq",
        args: "-e1 -q --native-frag --wrong-seq --blacklist {hostlist}",
    },
];

/// SpoofDPI 1.5. Most resolve names over HTTPS (DoH), which also gets past providers that
/// block by DNS; the last ones use the system's DNS for networks where DoH is blocked.
const SPOOFDPI: &[BuiltinStrategy] = &[
    BuiltinStrategy {
        name: "split at SNI + DoH",
        args: "--dns-mode https",
    },
    BuiltinStrategy {
        name: "split at SNI + disorder + DoH",
        args: "--dns-mode https --https-disorder",
    },
    BuiltinStrategy {
        name: "first byte + DoH",
        args: "--dns-mode https --https-split-mode first-byte",
    },
    BuiltinStrategy {
        name: "1-byte chunks + DoH",
        args: "--dns-mode https --https-split-mode chunk --https-chunk-size 1",
    },
    BuiltinStrategy {
        name: "random split + disorder + DoH",
        args: "--dns-mode https --https-split-mode random --https-disorder",
    },
    BuiltinStrategy {
        name: "fake packets + DoH",
        args: "--dns-mode https --https-fake-count 3",
    },
    BuiltinStrategy {
        name: "split at SNI (system DNS)",
        args: "--dns-mode system",
    },
    BuiltinStrategy {
        name: "split at SNI + disorder (system DNS)",
        args: "--dns-mode system --https-disorder",
    },
];

pub fn builtin_strategies(engine: EngineKind) -> &'static [BuiltinStrategy] {
    match engine {
        EngineKind::ByeDpi => BYEDPI,
        // nfqws gets the same set; `adapt_args` drops the WinDivert filters.
        EngineKind::ZapretWinws | EngineKind::ZapretNfqws => WINWS,
        EngineKind::GoodbyeDpi => GOODBYEDPI,
        EngineKind::ZapretTpws => TPWS,
        EngineKind::SpoofDpi => SPOOFDPI,
    }
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
    fn nfqws_strategies_lose_only_the_windivert_filters() {
        for s in builtin_strategies(EngineKind::ZapretNfqws) {
            let args = adapt_args(EngineKind::ZapretNfqws, s.args);
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
        for engine in [EngineKind::ZapretTpws, EngineKind::SpoofDpi] {
            for s in builtin_strategies(engine) {
                let args = crate::args::split_args(s.args);
                assert!(
                    crate::argpolicy::check_engine_args(engine, &args, &[&dir]).is_ok(),
                    "{}: {}",
                    engine.display_name(),
                    s.name
                );
            }
        }
    }
}
