pub fn with_stderr(base: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        base.to_string()
    } else {
        format!("{base}; stderr: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_base_when_stderr_is_empty() {
        assert_eq!(with_stderr("something failed", b""), "something failed");
    }

    #[test]
    fn returns_base_when_stderr_is_whitespace_only() {
        assert_eq!(
            with_stderr("something failed", b"  \n  "),
            "something failed"
        );
    }

    #[test]
    fn appends_stderr_detail_when_present() {
        assert_eq!(
            with_stderr("something failed", b"fatal: bad object\n"),
            "something failed; stderr: fatal: bad object"
        );
    }
}
