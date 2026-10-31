mod support;

use clawshell_lib::core::ports::{HttpClient, HttpRequest};
use std::time::{Duration, UNIX_EPOCH};
use support::Harness;

const ID_A: &str = "3f2b8c1e-5a4d-4e7b-9c10-2d6f8a9b0c11";
const ID_B: &str = "a0b1c2d3-e4f5-4a6b-8c7d-9e0f1a2b3c4d";

#[test]
fn scan_lists_only_clawshell_profile_folders_and_stamps_scan_time() {
    let h = Harness::new();
    h.fixture.add_profile(ID_A);
    h.fixture.add_profile(ID_B);
    // Folders clawshell did not create must never be listed.
    h.fixture.add_folder("ClaudeProfile-Work");
    h.fixture.add_folder("Claude");
    h.fixture.add_folder("clawshell"); // clawshell's own app data
    h.fixture.add_folder("clawshell-not-a-uuid");
    h.clock.advance(Duration::from_secs(90));

    let scan = h.core().scan_profiles().unwrap();

    let ids: Vec<_> = scan.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec![ID_A, ID_B]);
    assert_eq!(
        scan.profiles[0].path,
        h.fixture.local_app_data().join(format!("clawshell-{ID_A}"))
    );
    assert_eq!(
        scan.scanned_at,
        UNIX_EPOCH + Duration::from_secs(1_781_524_890)
    );
    assert!(
        h.http.requests().is_empty(),
        "a scan makes no network calls"
    );
}

#[test]
fn scripted_http_replays_responses_in_order_and_records_requests() {
    let h = Harness::new();
    h.http
        .respond("https://example.test/usage", 429, "slow down");
    h.http.respond("https://example.test/usage", 200, "{}");

    let ask = || {
        h.http
            .send(HttpRequest::get("https://example.test/usage"))
            .unwrap()
            .status
    };

    assert_eq!((ask(), ask()), (429, 200));
    assert_eq!(h.http.requests().len(), 2);
    assert!(
        h.http
            .send(HttpRequest::get("https://example.test/usage"))
            .is_err()
    );
}
