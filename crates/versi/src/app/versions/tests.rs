use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

use super::super::test_app_with_two_environments;
use super::*;
use crate::state::AppState;

fn remote(version: &str, is_latest: bool) -> versi_backend::RemoteVersion {
    versi_backend::RemoteVersion {
        version: version.parse().expect("test version should parse"),
        lts_codename: None,
        is_latest,
    }
}

fn sample_schedule() -> versi_core::ReleaseSchedule {
    serde_json::from_value(serde_json::json!({
        "versions": {
            "22": {
                "start": "2024-04-23",
                "lts": "2024-10-29",
                "maintenance": "2026-10-20",
                "end": "2027-04-30",
                "codename": "Jod"
            }
        }
    }))
    .expect("sample release schedule should deserialize")
}

fn sample_metadata() -> HashMap<String, versi_core::VersionMeta> {
    HashMap::from([(
        "v22.10.0".to_string(),
        versi_core::VersionMeta {
            date: "2026-01-01".to_string(),
            security: true,
            npm: Some("11.0.0".to_string()),
            v8: Some("12.0".to_string()),
            openssl: Some("3.4.0".to_string()),
        },
    )])
}

#[test]
fn remote_versions_fetched_updates_cache_on_success() {
    let mut app = test_app_with_two_environments();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.loading = true;
        state.available_versions.remote_request_seq = 7;
    }

    app.handle_remote_versions_fetched(
        7,
        Ok(vec![remote("v22.10.0", true), remote("v22.9.0", false)]),
    );

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(!state.available_versions.loading);
    assert!(state.available_versions.error.is_none());
    assert_eq!(state.available_versions.versions.len(), 2);
    assert_eq!(
        state.available_versions.latest_by_major.get(&22),
        Some(&"v22.10.0".parse().expect("version parse"))
    );
    assert!(state.available_versions.fetched_at.is_some());
    assert!(!state.available_versions.loaded_from_disk);
}

#[test]
fn release_schedule_fetched_ignores_stale_request() {
    let mut app = test_app_with_two_environments();
    let baseline = sample_schedule();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.schedule_request_seq = 3;
        state.available_versions.schedule = Some(baseline.clone());
    }

    app.handle_release_schedule_fetched(2, Ok(sample_schedule()));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert_eq!(
        state
            .available_versions
            .schedule
            .as_ref()
            .expect("baseline schedule should remain")
            .versions
            .len(),
        baseline.versions.len()
    );
}

#[test]
fn release_schedule_fetched_sets_schedule_and_clears_error() {
    let mut app = test_app_with_two_environments();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.schedule_request_seq = 5;
        state.available_versions.schedule_error = Some(AppError::version_fetch_failed(
            "Release schedule",
            "old error",
        ));
    }

    app.handle_release_schedule_fetched(5, Ok(sample_schedule()));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(state.available_versions.schedule.is_some());
    assert!(state.available_versions.schedule_error.is_none());
}

#[test]
fn version_metadata_fetched_ignores_stale_request() {
    let mut app = test_app_with_two_environments();
    let baseline = sample_metadata();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.metadata_request_seq = 4;
        state.available_versions.metadata = Some(baseline.clone());
    }

    app.handle_version_metadata_fetched(3, Ok(sample_metadata()));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert_eq!(
        state
            .available_versions
            .metadata
            .as_ref()
            .expect("baseline metadata should remain")
            .get("v22.10.0")
            .and_then(|meta| meta.npm.as_deref()),
        baseline
            .get("v22.10.0")
            .and_then(|meta| meta.npm.as_deref())
    );
}

#[test]
fn version_metadata_fetched_stores_metadata_on_success() {
    let mut app = test_app_with_two_environments();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.metadata_request_seq = 8;
        state.available_versions.metadata = None;
        state.available_versions.metadata_error = Some(AppError::version_fetch_failed(
            "Version metadata",
            "old error",
        ));
    }

    app.handle_version_metadata_fetched(8, Ok(sample_metadata()));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(state.available_versions.metadata.is_some());
    assert!(state.available_versions.metadata_error.is_none());
}

#[test]
fn version_metadata_fetched_stores_error_on_failure() {
    let mut app = test_app_with_two_environments();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.metadata_request_seq = 9;
        state.available_versions.metadata = None;
    }

    app.handle_version_metadata_fetched(
        9,
        Err(AppError::version_fetch_failed(
            "Version metadata",
            "metadata failed",
        )),
    );

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(matches!(
        state.available_versions.metadata_error,
        Some(AppError::VersionFetchFailed {
            resource: "Version metadata",
            ref details
        }) if details == "metadata failed"
    ));
}

#[test]
fn app_update_checked_sets_update_on_success() {
    let mut app = test_app_with_two_environments();
    let update = versi_core::AppUpdate {
        current_version: "0.9.0".to_string(),
        latest_version: "0.9.1".to_string(),
        release_url: "https://example.com/release".to_string(),
        release_notes: Some("notes".to_string()),
        download_url: Some("https://example.com/download".to_string()),
        download_size: Some(1234),
    };

    app.handle_app_update_checked(Ok(Some(update.clone())));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert_eq!(
        state
            .app_update
            .as_ref()
            .map(|value| value.latest_version.as_str()),
        Some("0.9.1")
    );
}

#[test]
fn backend_update_checked_sets_update_on_success() {
    let mut app = test_app_with_two_environments();
    let update = versi_backend::BackendUpdate {
        current_version: "1.0.0".to_string(),
        latest_version: "1.1.0".to_string(),
        release_url: "https://example.com/backend".to_string(),
    };

    app.handle_backend_update_checked(Ok(Some(update.clone())));

    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert_eq!(
        state
            .backend_update
            .as_ref()
            .map(|value| value.latest_version.as_str()),
        Some("1.1.0")
    );
}

#[test]
fn fetch_release_schedule_cancels_previous_token() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.schedule_cancel_token = Some(old_token.clone());
    }

    let _ = app.handle_fetch_release_schedule();

    assert!(old_token.is_cancelled());
    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(state.available_versions.schedule_cancel_token.is_some());
}

#[test]
fn fetch_version_metadata_cancels_previous_token() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.metadata_cancel_token = Some(old_token.clone());
    }

    let _ = app.handle_fetch_version_metadata();

    assert!(old_token.is_cancelled());
    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(state.available_versions.metadata_cancel_token.is_some());
}

#[test]
fn fetch_remote_versions_cancels_previous_token_when_loading() {
    let mut app = test_app_with_two_environments();
    let old_token = CancellationToken::new();
    if let AppState::Main(state) = &mut app.state {
        state.available_versions.loading = true;
        state.available_versions.remote_cancel_token = Some(old_token.clone());
    }

    let _ = app.handle_fetch_remote_versions();

    assert!(old_token.is_cancelled());
    let AppState::Main(state) = &app.state else {
        panic!("expected main state");
    };
    assert!(state.available_versions.loading);
    assert!(state.available_versions.remote_cancel_token.is_some());
}
