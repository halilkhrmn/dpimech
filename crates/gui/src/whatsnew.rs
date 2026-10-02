//! "What's new": after an update, the first start shows the release notes of every version since
//! the one that ran before. The notes are built in (changelog/<lang>.md, English as fallback), so
//! they work offline and in the user's language.

use crate::i18n;

const NOTES: &[(&str, &str)] = &[
    ("en", include_str!("../../../changelog/en.md")),
    ("tr", include_str!("../../../changelog/tr.md")),
    ("ru", include_str!("../../../changelog/ru.md")),
    ("fa", include_str!("../../../changelog/fa.md")),
    ("ar", include_str!("../../../changelog/ar.md")),
];

/// Versions shown at most, newest first: someone who skipped many releases gets the latest ones.
const MAX_VERSIONS: usize = 3;

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut parts = v.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    Some((next()?, next()?, next()?))
}

/// `## <version>` sections of a changelog, in file order (newest first).
fn sections(text: &str) -> Vec<((u32, u32, u32), String)> {
    let mut out: Vec<((u32, u32, u32), String)> = Vec::new();
    let mut current: Option<((u32, u32, u32), String)> = None;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            out.extend(current.take());
            current = parse_version(heading.split_whitespace().next().unwrap_or(""))
                .map(|v| (v, String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out.extend(current);
    out
}

/// The notes of the versions after `last` up to `current`, as plain text ("- " bullets become
/// "• "), or `None` when there is nothing to show. An empty `last` (the setting is new) shows
/// only the current version.
pub fn notes_since(text: &str, last: &str, current: &str) -> Option<String> {
    let current = parse_version(current)?;
    let last = parse_version(last);
    if last.is_some_and(|l| l >= current) {
        return None;
    }
    let shown: Vec<_> = sections(text)
        .into_iter()
        .filter(|(v, _)| *v <= current && last.map_or(*v == current, |l| *v > l))
        .take(MAX_VERSIONS)
        .collect();
    // The dialog's title names the current version; more than one gets a heading each.
    let headings = shown.len() > 1;
    let shown: Vec<String> = shown
        .into_iter()
        .map(|((a, b, c), body)| {
            let body = body
                .trim()
                .lines()
                .map(|l| match l.trim_start().strip_prefix("- ") {
                    Some(rest) => format!("•  {rest}"),
                    None => l.to_owned(),
                })
                .collect::<Vec<_>>()
                .join("\n");
            if headings {
                format!("{a}.{b}.{c}\n{body}")
            } else {
                body
            }
        })
        .collect();
    (!shown.is_empty()).then(|| shown.join("\n\n"))
}

/// What to show this start, in the current language (English when it has no entry for these
/// versions).
pub fn pending(last: &str) -> Option<String> {
    let current = dpimech_core::VERSION;
    let lang = i18n::current();
    let text = |code: &str| NOTES.iter().find(|(c, _)| *c == code).map(|(_, t)| *t);
    text(lang)
        .and_then(|t| notes_since(t, last, current))
        .or_else(|| notes_since(text("en")?, last, current))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str =
        "# Changelog\n\n## 0.3.0\n- New A\n- New B\n\n## 0.2.9\nFix C\n\n## 0.2.8\n- D\n";

    #[test]
    fn notes_since_last_start() {
        assert_eq!(
            notes_since(LOG, "0.2.8", "0.3.0").unwrap(),
            "0.3.0\n•  New A\n•  New B\n\n0.2.9\nFix C"
        );
        assert_eq!(notes_since(LOG, "", "0.3.0").unwrap(), "•  New A\n•  New B");
        assert_eq!(notes_since(LOG, "0.3.0", "0.3.0"), None);
        assert_eq!(notes_since(LOG, "0.3.1", "0.3.0"), None); // went back a version
        assert_eq!(
            notes_since(LOG, "0.2.9", "0.3.1"),
            Some("•  New A\n•  New B".into())
        );
        assert_eq!(notes_since(LOG, "", "0.4.0"), None); // no entry for this version
    }

    #[test]
    fn every_language_has_the_current_version() {
        for (lang, text) in NOTES {
            assert!(
                notes_since(text, "", dpimech_core::VERSION).is_some(),
                "changelog/{lang}.md has no section for {}",
                dpimech_core::VERSION
            );
        }
    }
}
