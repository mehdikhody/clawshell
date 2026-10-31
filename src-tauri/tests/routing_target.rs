use clawshell_lib::core::routing::{Candidate, Target, pick_target};

fn cand(id: &str) -> Candidate {
    Candidate {
        id: id.to_string(),
        signed_out: false,
        running: false,
        launched_signed_out: None,
        last_used: None,
    }
}

fn account(id: &str) -> Target {
    Target::Account(id.to_string())
}

#[test]
fn the_account_launched_while_signed_out_comes_first() {
    let mut a = cand("a");
    a.signed_out = true;
    a.launched_signed_out = Some(1);
    // A different account was used more recently and is the only open signed-out one.
    let mut b = cand("b");
    b.signed_out = true;
    b.running = true;
    b.last_used = Some(9);
    assert_eq!(pick_target(&[a, b]), account("a"));
}

#[test]
fn the_latest_signed_out_launch_wins_over_an_earlier_one() {
    let mut a = cand("a");
    a.signed_out = true;
    a.launched_signed_out = Some(3);
    let mut b = cand("b");
    b.signed_out = true;
    b.launched_signed_out = Some(7);
    assert_eq!(pick_target(&[a, b]), account("b"));
}

#[test]
fn an_account_that_signed_in_since_its_launch_is_skipped() {
    let mut a = cand("a");
    a.launched_signed_out = Some(5); // signed_out is false now
    a.running = true;
    a.last_used = Some(5);
    let mut b = cand("b");
    b.signed_out = true;
    b.running = true;
    assert_eq!(pick_target(&[a, b]), account("b"));
}

#[test]
fn the_only_open_signed_out_account_comes_second() {
    let mut a = cand("a");
    a.signed_out = true;
    a.running = true;
    let mut b = cand("b");
    b.running = true;
    b.last_used = Some(4); // used last, but signed in
    let mut c = cand("c");
    c.signed_out = true; // signed out but closed
    assert_eq!(pick_target(&[a, b, c]), account("a"));
}

#[test]
fn two_open_signed_out_accounts_fall_through_to_the_window_used_last() {
    let mut a = cand("a");
    a.signed_out = true;
    a.running = true;
    let mut b = cand("b");
    b.signed_out = true;
    b.running = true;
    // Neither is "the only" one, so the last-used window decides.
    b.last_used = Some(2);
    assert_eq!(pick_target(&[a, b]), account("b"));
}

#[test]
fn the_window_used_last_comes_third() {
    let mut a = cand("a");
    a.running = true;
    a.last_used = Some(2);
    let mut b = cand("b");
    b.running = true;
    b.last_used = Some(6);
    let mut c = cand("c");
    c.last_used = Some(9); // used most recently, but its window is closed
    assert_eq!(pick_target(&[a, b, c]), account("b"));
}

#[test]
fn the_original_claude_install_is_the_last_resort() {
    assert_eq!(pick_target(&[]), Target::OriginalClaude);
    let mut idle = cand("a");
    idle.running = true; // open, but never used through clawshell
    assert_eq!(pick_target(&[idle]), Target::OriginalClaude);
}
