/// Splits a command-line string into arguments, honouring double and single quotes.
/// Backslashes are kept literally so Windows paths survive unchanged.
pub fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;

    for c in input.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                in_token = true;
            }
            None if c.is_whitespace() => {
                if in_token {
                    out.push(std::mem::take(&mut cur));
                    in_token = false;
                }
            }
            None => {
                cur.push(c);
                in_token = true;
            }
        }
    }
    if in_token {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split_args;

    #[test]
    fn splits_plain_and_quoted() {
        assert_eq!(split_args("-r 1+s"), ["-r", "1+s"]);
        assert_eq!(
            split_args(r#"--hostlist "C:\my lists\a.txt"  -x ''"#),
            ["--hostlist", r"C:\my lists\a.txt", "-x", ""]
        );
        assert!(split_args("   ").is_empty());
    }
}
