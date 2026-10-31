use clawshell_lib::core::link::{ClaudeLink, LinkError};

#[test]
fn accepts_a_claude_link_and_hands_it_back_unchanged() {
    let raw = "claude://login/callback?code=abc123&state=xyz";
    let link = ClaudeLink::parse(raw).unwrap();
    assert_eq!(link.as_str(), raw);
}

#[test]
fn the_scheme_is_matched_without_regard_to_case() {
    assert!(ClaudeLink::parse("CLAUDE://login/callback?code=1").is_ok());
    assert!(ClaudeLink::parse("Claude://login").is_ok());
}

#[test]
fn refuses_other_schemes() {
    for raw in [
        "https://claude.ai/login",
        "file:///C:/Windows/System32/calc.exe",
        "claude-desktop://login",
        "javascript:alert(1)",
        "claude:login",
        "claude:/login",
        "login?claude://x",
        "C:\\Windows\\notepad.exe",
        "--user-data-dir=C:\\x",
        "",
    ] {
        assert!(
            matches!(ClaudeLink::parse(raw), Err(LinkError::NotClaudeLink)),
            "{raw:?} should be refused"
        );
    }
}

#[test]
fn refuses_a_link_with_nothing_after_the_scheme() {
    assert!(matches!(
        ClaudeLink::parse("claude://"),
        Err(LinkError::Malformed)
    ));
}

#[test]
fn refuses_malformed_links() {
    for raw in [
        "claude://login callback",
        "claude://login\tcallback",
        "claude://login\ncallback",
        "claude://login\0",
        "claude://login\"--flag",
        "claude://login<script>",
        "claude://login>out",
        "claude://login\\evil",
        "claude://login^x",
        "claude://login|calc",
        "claude://login`x",
        "claude://lögin",
        "claude://login\u{202e}",
    ] {
        assert!(
            matches!(ClaudeLink::parse(raw), Err(LinkError::Malformed)),
            "{raw:?} should be refused"
        );
    }
}

#[test]
fn refuses_a_link_longer_than_a_browser_would_send() {
    let long = format!("claude://login?{}", "a".repeat(8_200));
    assert!(matches!(
        ClaudeLink::parse(&long),
        Err(LinkError::Malformed)
    ));
}

#[test]
fn neither_the_link_nor_its_errors_print_the_contents() {
    let secret = "claude://login/callback?code=s3cr3t-one-time-code";
    let link = ClaudeLink::parse(secret).unwrap();
    assert!(!format!("{link:?}").contains("s3cr3t"));

    let bad = "claude://login s3cr3t";
    let error = ClaudeLink::parse(bad).unwrap_err();
    assert!(!format!("{error:?}").contains("s3cr3t"));
    assert!(!error.to_string().contains("s3cr3t"));
}
