//! Translations for text built in Rust (status lines, wizard steps, notifications, tray).
//!
//! Slint's `@tr()` strings are bundled by build.rs from the same `.po` files
//! (`lang/<lang>/LC_MESSAGES/dpimech-gui.po`), so there is one catalog per language for
//! translators. Placeholders follow Slint: `{}` in order, or `{0}`, `{1}` when a language
//! needs another word order.

/// `trf!("{} of {}", a, b)`: translate the template, then fill in the arguments.
macro_rules! trf {
    ($template:expr $(, $arg:expr)* $(,)?) => {
        $crate::i18n::trf($template, &[$(&$arg as &dyn ::std::fmt::Display),*])
    };
}

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::RwLock;

/// Languages with a catalog, besides English (the source language).
pub const LANGUAGES: &[(&str, &str)] = &[("tr", "Türkçe"), ("ru", "Русский")];

const CATALOGS: &[(&str, &str)] = &[
    ("tr", include_str!("../lang/tr/LC_MESSAGES/dpimech-gui.po")),
    ("ru", include_str!("../lang/ru/LC_MESSAGES/dpimech-gui.po")),
];

/// Texts that reach `tr()` through a variable (sent by the service, or names from the core
/// crate); listed here so the catalog extractor (tools/i18n.py) picks them up.
// Read by tools/i18n.py and the tests, not by the program itself.
#[cfg_attr(not(test), allow(dead_code))]
pub const EXTRA_TEXTS: &[&str] = &[
    "Automatic",
    "Restarting did not help. This setting may have stopped working on your connection — run the Strategy Lab.",
    "This setting does not open this profile's sites on your connection. Find a working one in the Strategy Lab, or try another engine.",
    "Per-app",
    "System-wide",
    "Local proxy",
];

static CURRENT: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Switches Rust-side text to `lang` ("tr", "ru"; anything else is English) and returns the
/// language actually used.
pub fn set_language(lang: &str) -> &'static str {
    let found = CATALOGS.iter().find(|(code, _)| *code == lang);
    *CURRENT.write().unwrap() = found.map(|(_, po)| parse_po(po));
    found.map_or("en", |(code, _)| code)
}

/// The user's language from the system, reduced to one we have ("tr-TR" → "tr").
pub fn system_language() -> String {
    let raw = system_locale().unwrap_or_default().to_ascii_lowercase();
    let base = raw.split(['_', '-', '.', '@']).next().unwrap_or("");
    if LANGUAGES.iter().any(|(code, _)| *code == base) {
        base.to_owned()
    } else {
        "en".to_owned()
    }
}

#[cfg(windows)]
fn system_locale() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Control Panel\International")
        .ok()?
        .get_value("LocaleName")
        .ok()
}

/// Apps started from Finder get no `LANG`; the user's language list is in the defaults.
#[cfg(target_os = "macos")]
fn system_locale() -> Option<String> {
    posix_locale().or_else(|| {
        let out = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
            .ok()?;
        // "(\n    \"tr-TR\",\n    en\n)" → first entry
        String::from_utf8_lossy(&out.stdout)
            .split(['(', ',', '\n'])
            .map(|s| s.trim().trim_matches('"').to_owned())
            .find(|s| !s.is_empty() && s != ")")
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_locale() -> Option<String> {
    posix_locale()
}

#[cfg(not(windows))]
fn posix_locale() -> Option<String> {
    // POSIX order of precedence; LANGUAGE may list several ("tr:en").
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|v| v.split(':').next().unwrap_or("").to_owned())
        .find(|v| !v.is_empty() && v != "C" && v != "POSIX")
}

/// Applies `choice` ("" = system language) to Slint's bundled translations and to Rust-side
/// text; returns the language in use.
pub fn apply(choice: &str) -> &'static str {
    let wanted = if choice.is_empty() {
        system_language()
    } else {
        choice.to_owned()
    };
    let lang = set_language(&wanted);
    // "" selects the source language (English); must run after the window exists.
    let _ = slint::select_bundled_translation(if lang == "en" { "" } else { lang });
    lang
}

/// Choices shown in Settings, in order: automatic, English, then every catalog.
pub fn choices() -> Vec<(&'static str, &'static str)> {
    let mut out = vec![("", "Automatic"), ("en", "English")];
    out.extend(LANGUAGES.iter().copied());
    out
}

/// The translation of `text`, or `text` itself.
pub fn tr(text: &str) -> String {
    CURRENT
        .read()
        .unwrap()
        .as_ref()
        .and_then(|m| m.get(text))
        .filter(|t| !t.is_empty())
        .cloned()
        .unwrap_or_else(|| text.to_owned())
}

/// Translates `template`, then fills its placeholders.
pub fn trf(template: &str, args: &[&dyn Display]) -> String {
    fill(&tr(template), args)
}

fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if after[..end].chars().all(|c| c.is_ascii_digit()) => {
                let index = if end == 0 {
                    next += 1;
                    next - 1
                } else {
                    after[..end].parse().unwrap_or(usize::MAX)
                };
                match args.get(index) {
                    Some(arg) => out.push_str(&arg.to_string()),
                    None => out.push_str(&rest[start..start + end + 2]),
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// msgid → msgstr of a gettext catalog (no plurals or contexts are used here).
fn parse_po(text: &str) -> HashMap<String, String> {
    #[derive(PartialEq)]
    enum Field {
        None,
        Id,
        Str,
    }
    let mut map = HashMap::new();
    let (mut id, mut msg, mut field) = (String::new(), String::new(), Field::None);
    let mut flush = |id: &mut String, msg: &mut String| {
        if !id.is_empty() && !msg.is_empty() {
            map.insert(std::mem::take(id), std::mem::take(msg));
        }
        id.clear();
        msg.clear();
    };
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("msgid ") {
            flush(&mut id, &mut msg);
            id = unquote(rest);
            field = Field::Id;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            msg = unquote(rest);
            field = Field::Str;
        } else if line.starts_with('"') {
            match field {
                Field::Id => id.push_str(&unquote(line)),
                Field::Str => msg.push_str(&unquote(line)),
                Field::None => {}
            }
        } else if line.is_empty() || line.starts_with('#') {
            field = Field::None;
        }
    }
    flush(&mut id, &mut msg);
    map
}

fn unquote(s: &str) -> String {
    // Exactly one quote on each side: the content itself may end in an escaped `\"`.
    let s = s.trim();
    let inner = s.strip_prefix('"').unwrap_or(s);
    let inner = inner.strip_suffix('"').unwrap_or(inner);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_in_order_and_by_index() {
        assert_eq!(fill("{} of {}", &[&1, &2]), "1 of 2");
        assert_eq!(fill("{1} / {0}", &[&"a", &"b"]), "b / a");
        assert_eq!(fill("{name} {}", &[&3]), "{name} 3");
        assert_eq!(fill("{} {}", &[&1]), "1 {}");
    }

    #[test]
    fn parses_multiline_entries_and_escapes() {
        let po = "msgid \"\"\nmsgstr \"Content-Type: text/plain\\n\"\n\n#: a.slint\nmsgid \"Say \\\"hi\\\"\"\nmsgstr \"\"\n\"Merhaba \"\n\"de\"\n\nmsgid \"Empty\"\nmsgstr \"\"\n";
        let map = parse_po(po);
        assert_eq!(
            map.get("Say \"hi\"").map(String::as_str),
            Some("Merhaba de")
        );
        assert!(
            !map.contains_key("Empty"),
            "untranslated entries fall back to English"
        );
        assert!(!map.contains_key(""), "header skipped");
    }

    #[test]
    fn texts_passed_in_variables_are_translated() {
        for (lang, po) in CATALOGS {
            let map = parse_po(po);
            for text in EXTRA_TEXTS {
                assert!(map.contains_key(*text), "{lang}: {text:?} missing");
            }
        }
    }

    #[test]
    fn every_catalog_parses_and_keeps_placeholders() {
        for (lang, po) in CATALOGS {
            let map = parse_po(po);
            assert!(!map.is_empty(), "{lang} catalog is empty");
            for (id, msg) in &map {
                let count = |s: &str| s.matches('{').count();
                assert_eq!(
                    count(id),
                    count(msg),
                    "{lang}: placeholders differ in {id:?}"
                );
            }
        }
    }
}
