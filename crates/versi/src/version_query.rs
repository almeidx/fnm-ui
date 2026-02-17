use std::collections::{HashMap, HashSet};

use versi_backend::{NodeVersion, RemoteVersion};
use versi_core::ReleaseSchedule;

use crate::state::SearchFilter;

pub(crate) struct AvailableVersionSearch<'a> {
    pub(crate) versions: Vec<&'a RemoteVersion>,
    pub(crate) alias_resolved: bool,
}

pub(crate) fn resolve_alias<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
) -> Option<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();

    match query_lower.as_str() {
        "latest" | "stable" | "current" => versions.iter().max_by_key(|v| &v.version),
        "lts/*" => versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .max_by_key(|v| &v.version),
        q if q.starts_with("lts/") => {
            let codename = &q[4..];
            versions
                .iter()
                .filter(|v| {
                    v.lts_codename
                        .as_ref()
                        .is_some_and(|c| c.to_lowercase() == codename)
                })
                .max_by_key(|v| &v.version)
        }
        _ => None,
    }
}

pub(crate) fn search_available_versions<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
    limit: usize,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) -> AvailableVersionSearch<'a> {
    let query_lower = query.to_lowercase();

    if let Some(resolved) = resolve_alias(versions, query) {
        let filtered = if matches_active_filters(resolved, active_filters, installed_set, schedule)
        {
            vec![resolved]
        } else {
            Vec::new()
        };
        return AvailableVersionSearch {
            versions: filtered,
            alias_resolved: true,
        };
    }

    let mut result = if query_lower == "lts" {
        latest_by_major(versions.iter().filter(|v| v.lts_codename.is_some()))
    } else {
        latest_by_minor(
            versions
                .iter()
                .filter(|v| matches_remote_query(v, query, &query_lower)),
        )
    };
    apply_active_filters(&mut result, active_filters, installed_set, schedule);
    result.truncate(limit);

    AvailableVersionSearch {
        versions: result,
        alias_resolved: false,
    }
}

fn matches_remote_query(version: &RemoteVersion, query: &str, query_lower: &str) -> bool {
    if query_lower == "lts" {
        return version.lts_codename.is_some();
    }

    let version_str = version.version.to_string();
    version_str.contains(query)
        || version
            .lts_codename
            .as_ref()
            .is_some_and(|c| c.to_lowercase().contains(query_lower))
}

fn latest_by_major<'a>(
    versions: impl Iterator<Item = &'a RemoteVersion>,
) -> Vec<&'a RemoteVersion> {
    let mut latest_by_major: HashMap<u32, &RemoteVersion> = HashMap::new();

    for version in versions {
        latest_by_major
            .entry(version.version.major)
            .and_modify(|existing| {
                if version.version > existing.version {
                    *existing = version;
                }
            })
            .or_insert(version);
    }

    let mut result: Vec<&RemoteVersion> = latest_by_major.into_values().collect();
    result.sort_by(|a, b| b.version.cmp(&a.version));
    result
}

fn latest_by_minor<'a>(
    versions: impl Iterator<Item = &'a RemoteVersion>,
) -> Vec<&'a RemoteVersion> {
    let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();

    for version in versions {
        let key = (version.version.major, version.version.minor);
        latest_by_minor
            .entry(key)
            .and_modify(|existing| {
                if version.version.patch > existing.version.patch {
                    *existing = version;
                }
            })
            .or_insert(version);
    }

    let mut result: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
    result.sort_by(|a, b| b.version.cmp(&a.version));
    result
}

fn apply_active_filters(
    versions: &mut Vec<&RemoteVersion>,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) {
    if active_filters.is_empty() {
        return;
    }

    versions
        .retain(|version| matches_active_filters(version, active_filters, installed_set, schedule));
}

fn matches_active_filters(
    version: &RemoteVersion,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<NodeVersion>,
    schedule: Option<&ReleaseSchedule>,
) -> bool {
    if active_filters.contains(&SearchFilter::Lts) && version.lts_codename.is_none() {
        return false;
    }

    if active_filters.contains(&SearchFilter::Installed)
        && !installed_set.contains(&version.version)
    {
        return false;
    }

    if active_filters.contains(&SearchFilter::NotInstalled)
        && installed_set.contains(&version.version)
    {
        return false;
    }

    if active_filters.contains(&SearchFilter::Eol) {
        let is_eol = schedule.is_some_and(|s| !s.is_active(version.version.major));
        if !is_eol {
            return false;
        }
    }

    if active_filters.contains(&SearchFilter::Active) {
        let is_active = schedule.is_none_or(|s| s.is_active(version.version.major));
        if !is_active {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    use super::{resolve_alias, search_available_versions};
    use crate::state::SearchFilter;

    fn remote(version: &str, lts_codename: Option<&str>) -> versi_backend::RemoteVersion {
        versi_backend::RemoteVersion {
            version: version.parse().expect("test version should parse"),
            lts_codename: lts_codename.map(str::to_string),
            is_latest: false,
        }
    }

    fn schedule_with_eol_major(eol_major: u32) -> versi_core::ReleaseSchedule {
        serde_json::from_value(serde_json::json!({
            "versions": {
                format!("{eol_major}"): {
                    "start": "2020-01-01",
                    "end": "2021-01-01"
                },
                "22": {
                    "start": "2024-04-23",
                    "lts": "2024-10-29",
                    "maintenance": "2026-10-20",
                    "end": "2027-04-30",
                    "codename": "Jod"
                }
            }
        }))
        .expect("schedule fixture should deserialize")
    }

    #[test]
    fn alias_latest_resolves_to_highest_version() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];
        let resolved = resolve_alias(&versions, "latest").expect("alias should resolve");
        assert_eq!(resolved.version.to_string(), "v22.1.0");
    }

    #[test]
    fn query_results_include_alias_resolution_flag() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];
        let search = search_available_versions(
            &versions,
            "stable",
            20,
            &HashSet::new(),
            &HashSet::new(),
            None,
        );

        assert!(search.alias_resolved);
        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v22.1.0");
    }

    #[test]
    fn query_filters_apply_installed_and_eol_constraints() {
        let versions = vec![
            remote("v22.1.0", Some("Jod")),
            remote("v20.11.0", Some("Iron")),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(20, 11, 0)]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Eol]);
        let schedule = schedule_with_eol_major(20);

        let search =
            search_available_versions(&versions, "v", 20, &filters, &installed, Some(&schedule));
        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v20.11.0");
        assert!(!search.alias_resolved);
    }

    #[test]
    fn limit_is_applied_after_filters_to_fill_result_window() {
        let versions = vec![
            remote("v22.3.0", None),
            remote("v22.2.0", None),
            remote("v22.1.0", None),
        ];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 2, 0)]);
        let filters = HashSet::from([SearchFilter::Installed]);

        let search = search_available_versions(&versions, "v22", 1, &filters, &installed, None);

        assert_eq!(search.versions.len(), 1);
        assert_eq!(search.versions[0].version.to_string(), "v22.2.0");
    }

    #[test]
    fn alias_resolution_respects_active_filters() {
        let versions = vec![remote("v22.1.0", Some("Jod")), remote("v20.11.0", None)];
        let installed = HashSet::from([versi_backend::NodeVersion::new(22, 1, 0)]);
        let filters = HashSet::from([SearchFilter::NotInstalled]);

        let search = search_available_versions(&versions, "stable", 10, &filters, &installed, None);

        assert!(search.alias_resolved);
        assert!(search.versions.is_empty());
    }

    #[test]
    #[ignore = "performance baseline; run manually"]
    fn perf_search_available_versions_large_dataset() {
        let mut versions = Vec::new();
        for major in 18_u32..=28 {
            for minor in 0_u32..40 {
                for patch in 0_u32..3 {
                    versions.push(versi_backend::RemoteVersion {
                        version: versi_backend::NodeVersion::new(major, minor, patch),
                        lts_codename: if major % 2 == 0 {
                            Some(format!("LTS-{major}"))
                        } else {
                            None
                        },
                        is_latest: patch == 2,
                    });
                }
            }
        }
        let installed = HashSet::from([
            versi_backend::NodeVersion::new(20, 39, 2),
            versi_backend::NodeVersion::new(22, 39, 2),
            versi_backend::NodeVersion::new(24, 39, 2),
        ]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Active]);
        let schedule = schedule_with_eol_major(20);

        let started = Instant::now();
        for _ in 0..200 {
            let result = search_available_versions(
                &versions,
                "v2",
                30,
                &filters,
                &installed,
                Some(&schedule),
            );
            std::hint::black_box(result.versions.len());
        }
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "search baseline exceeded: {elapsed:?}"
        );
    }
}
