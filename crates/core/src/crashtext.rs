//! The text a crash leaves behind, shared by the app and the service: what panicked, where, in
//! which thread, and the backtrace without the panic machinery's own frames.

/// Keeps the record readable in the report dialog and below the size a link can carry.
pub const MAX_TEXT: usize = 6_000;

/// One crash record. `when` is already formatted by the caller (the service has no clock crate).
pub fn describe(program: &str, when: &str, thread: &str, panic: &str, backtrace: &str) -> String {
    format!(
        "{program} {} crashed at {when} (thread \"{thread}\")\n{panic}\nBacktrace:\n{}\n",
        crate::VERSION,
        trim_backtrace(backtrace),
    )
}

/// Drops the frames before the panic itself (the hook, `begin_panic`, `rust_begin_unwind`, …):
/// the first frame worth reading is the code that panicked.
pub fn trim_backtrace(backtrace: &str) -> String {
    let lines: Vec<&str> = backtrace.lines().collect();
    let last_machinery = lines.iter().rposition(|l| {
        let l = l.trim_start();
        is_frame(l)
            && [
                "begin_unwind",
                "panic_fmt",
                "begin_panic",
                "panicking::panic",
            ]
            .iter()
            .any(|m| l.contains(m))
    });
    let Some(at) = last_machinery else {
        return backtrace.to_owned();
    };
    // Skip that frame's "at file:line" line too.
    let start = lines[at + 1..]
        .iter()
        .position(|l| is_frame(l.trim_start()))
        .map_or(lines.len(), |p| at + 1 + p);
    lines[start..].join("\n")
}

/// Frame lines start with their number: `  12: name`.
fn is_frame(line: &str) -> bool {
    line.split_once(':')
        .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Appends `text` to an earlier record (later panics are usually caused by the first) and keeps
/// the whole within `MAX_TEXT`, cut on a character boundary.
pub fn append(old: &str, text: &str) -> String {
    let mut all = format!("{old}{text}");
    if all.len() > MAX_TEXT {
        let mut end = MAX_TEXT;
        while !all.is_char_boundary(end) {
            end -= 1;
        }
        all.truncate(end);
        all.push_str("\n[…]\n");
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    const BACKTRACE: &str = "   0: dpimech::crash::install::{{closure}}
             at ./crates/gui/src/crash.rs:25:18
   1: std::panicking::rust_panic_with_hook
             at /rustc/x/library/std/src/panicking.rs:833:13
   2: __rustc::rust_begin_unwind
             at /rustc/x/library/std/src/panicking.rs:689:5
   3: core::panicking::panic_fmt
             at /rustc/x/library/core/src/panicking.rs:80:14
   4: dpimech::main
             at ./crates/gui/src/main.rs:73:9
   5: core::ops::function::FnOnce::call_once";

    #[test]
    fn backtrace_starts_at_the_code_that_panicked() {
        let trimmed = trim_backtrace(BACKTRACE);
        assert!(
            trimmed.trim_start().starts_with("4: dpimech::main"),
            "{trimmed}"
        );
        assert!(trimmed.contains("main.rs:73:9"));
        assert!(!trimmed.contains("rust_begin_unwind"));
        // Nothing recognisable (e.g. a stripped build): kept as it is.
        assert_eq!(trim_backtrace("<unknown>"), "<unknown>");
    }

    #[test]
    fn record_names_program_version_thread_and_panic() {
        let text = describe(
            "DPIMech",
            "2026-11-24 22:00:00",
            "ipc",
            "panicked at x.rs:1:1:\nboom",
            BACKTRACE,
        );
        assert!(text.starts_with(&format!(
            "DPIMech {} crashed at 2026-11-24 22:00:00 (thread \"ipc\")",
            crate::VERSION
        )));
        assert!(text.contains("boom"));
    }

    #[test]
    fn long_records_are_cut_on_a_character_boundary() {
        let cut = append("", &"ğ".repeat(MAX_TEXT));
        assert!(cut.len() <= MAX_TEXT + 8);
        assert!(cut.ends_with("[…]\n"));
        assert_eq!(append("a", "b"), "ab");
    }
}
