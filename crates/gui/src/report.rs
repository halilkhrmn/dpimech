//! "Report a problem": a plain-text report the user reviews first, then sends as a GitHub
//! issue or by e-mail. Links cannot carry much text (browsers and mail programs cut long
//! URLs), so the full report is saved to a file and the link carries a summary plus the
//! newest log lines.

use std::fmt::Write as _;
use std::path::PathBuf;

use dpimech_core::catalog::{APP_REPO, SUPPORT_EMAIL};
use slint::Model;

use crate::AppWindow;
use crate::state::PROFILES;

/// Log lines in the report file (newest last).
const FILE_LOG_LINES: usize = 300;
/// A GitHub "new issue" link longer than this is rejected, so the body is trimmed to fit.
const MAX_ISSUE_URL: usize = 7_500;
/// Some mail programs (and Windows' URL handling) cut `mailto:` links around 2 000 characters.
const MAX_MAILTO_URL: usize = 1_800;

/// The report shown in the dialog and where its full text was saved.
pub type Open = Option<(Report, Option<PathBuf>)>;

pub struct Report {
    /// Version, system, service and profiles.
    pub summary: String,
    /// Log lines, oldest first.
    pub log: Vec<String>,
}

impl Report {
    pub fn collect(ui: &AppWindow) -> Report {
        let mut summary = String::new();
        let _ = writeln!(summary, "DPIMech {}", ui.get_app_version());
        let _ = writeln!(summary, "System: {}", os_description());
        let _ = writeln!(
            summary,
            "Service: {}{}",
            if ui.get_service_connected() {
                "connected"
            } else {
                "NOT running"
            },
            match ui.get_service_info().as_str() {
                "" => String::new(),
                info => format!(" ({info})"),
            }
        );
        let packages = ui.get_packages();
        let installed: Vec<String> = packages
            .iter()
            .filter(|p| !p.installed.is_empty())
            .map(|p| format!("{} {}", p.name, p.installed))
            .collect();
        let _ = writeln!(
            summary,
            "Engines: {}",
            if installed.is_empty() {
                "none installed".to_owned()
            } else {
                installed.join(", ")
            }
        );
        let _ = writeln!(summary, "Profiles:");
        PROFILES.with_borrow(|profiles| {
            if profiles.is_empty() {
                let _ = writeln!(summary, "  (none)");
            }
            for p in profiles {
                let _ = writeln!(
                    summary,
                    "  - {} · {} · {} · {:?}\n    args: {}",
                    p.profile.name,
                    p.profile.engine.display_name(),
                    p.profile.routing.mode().display_name(),
                    p.status,
                    p.profile.args
                );
            }
        });

        // The log model is newest first; reports read top to bottom.
        let logs = ui.get_logs();
        let mut log: Vec<String> = logs
            .iter()
            .take(FILE_LOG_LINES)
            .map(|l| {
                format!(
                    "{} {:5} [{}] {}",
                    l.time,
                    l.level.to_uppercase(),
                    l.source,
                    l.text
                )
            })
            .collect();
        log.reverse();
        Report { summary, log }
    }

    pub fn text(&self, note: &str) -> String {
        let mut out = String::new();
        if !note.trim().is_empty() {
            let _ = writeln!(out, "What happened: {}\n", note.trim());
        }
        out.push_str(&self.summary);
        let _ = writeln!(out, "\nRecent log ({} lines):", self.log.len());
        for line in &self.log {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    /// Saves the full report next to the log files and returns its path.
    pub fn save(&self, note: &str) -> Option<PathBuf> {
        let dir = crate::logfile::default_dir()?;
        std::fs::create_dir_all(&dir).ok()?;
        let name = format!(
            "dpimech-report-{}.txt",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let path = dir.join(name);
        std::fs::write(&path, self.text(note)).ok()?;
        Some(path)
    }

    pub fn github_url(&self, note: &str, saved: Option<&PathBuf>) -> String {
        let title = match note.trim() {
            "" => "Problem report".to_owned(),
            n => n.chars().take(80).collect(),
        };
        let base = format!(
            "https://github.com/{APP_REPO}/issues/new?title={}&body=",
            encode(&title)
        );
        let attach = saved
            .map(|p| {
                format!(
                    "\n_Please drag the full report into this issue: `{}`_\n",
                    p.display()
                )
            })
            .unwrap_or_default();
        let fit = |lines: usize| {
            let start = self.log.len().saturating_sub(lines);
            format!(
                "{}\n```\n{}```\n{attach}\n**Newest log lines**\n```\n{}\n```\n",
                if note.trim().is_empty() {
                    "_Describe what happened here._\n".to_owned()
                } else {
                    format!("{}\n", note.trim())
                },
                self.summary,
                self.log[start..].join("\n")
            )
        };
        base.clone() + &encode(&longest_fitting(&base, MAX_ISSUE_URL, self.log.len(), fit))
    }

    pub fn mailto_url(&self, note: &str, saved: Option<&PathBuf>) -> String {
        let base = format!(
            "mailto:{SUPPORT_EMAIL}?subject={}&body=",
            encode("DPIMech problem report")
        );
        let attach = saved
            .map(|p| format!("Please attach the full report: {}\n\n", p.display()))
            .unwrap_or_default();
        let fit = |lines: usize| {
            let start = self.log.len().saturating_sub(lines);
            format!(
                "{}\n\n{attach}{}\nNewest log lines:\n{}\n",
                note.trim(),
                self.summary,
                self.log[start..].join("\n")
            )
        };
        base.clone() + &encode(&longest_fitting(&base, MAX_MAILTO_URL, self.log.len(), fit))
    }
}

/// The body with as many of the newest log lines as fit into `max` URL characters.
fn longest_fitting(base: &str, max: usize, lines: usize, body: impl Fn(usize) -> String) -> String {
    (0..=lines)
        .rev()
        .map(&body)
        .find(|b| base.len() + encode(b).len() <= max)
        .unwrap_or_else(|| body(0).chars().take(max / 4).collect())
}

/// Percent-encoding for URL query values (RFC 3986 unreserved characters stay as they are).
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

/// "Windows 11 (26100)", "Ubuntu 24.04 LTS", …
fn os_description() -> String {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        {
            let build: String = key.get_value("CurrentBuildNumber").unwrap_or_default();
            let display: String = key.get_value("DisplayVersion").unwrap_or_default();
            // ProductName still says "Windows 10" on Windows 11; the build number tells.
            let name = if build.parse::<u32>().is_ok_and(|b| b >= 22000) {
                "Windows 11"
            } else {
                "Windows 10"
            };
            return format!("{name} {display} (build {build})");
        }
        "Windows".to_owned()
    }
    #[cfg(target_os = "macos")]
    {
        let version = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default();
        format!("macOS {version}")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                    .map(|v| v.trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| std::env::consts::OS.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(lines: usize) -> Report {
        Report {
            summary: "DPIMech 0.2.0\nSystem: Test OS\n".into(),
            log: (0..lines)
                .map(|i| {
                    format!(
                        "12:00:{:02} INFO  [Discord] line {i} with ünïcode & ?=#",
                        i % 60
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn links_fit_and_keep_the_newest_lines() {
        let r = report(500);
        let issue = r.github_url("Discord stopped", None);
        assert!(issue.len() <= MAX_ISSUE_URL, "{}", issue.len());
        assert!(issue.contains("title=Discord%20stopped"));
        assert!(issue.contains(&encode("line 499 ")), "newest line kept");
        assert!(!issue.contains(&encode("line 0 ")), "oldest lines dropped");

        let mail = r.mailto_url("", Some(&PathBuf::from("/tmp/r.txt")));
        assert!(mail.len() <= MAX_MAILTO_URL, "{}", mail.len());
        assert!(mail.starts_with(
            "mailto:halilkahraman@yandex.com?subject=DPIMech%20problem%20report&body="
        ));
        assert!(mail.contains(&encode("/tmp/r.txt")));
    }

    #[test]
    fn encodes_reserved_characters() {
        assert_eq!(encode("a b&c=d?é"), "a%20b%26c%3Dd%3F%C3%A9");
    }
}
