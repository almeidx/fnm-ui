use std::collections::{HashMap, HashSet};

use versi_backend::RemoteVersion;
use versi_core::ReleaseSchedule;

use crate::state::SearchFilter;

pub(super) fn resolve_alias<'a>(
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

pub(super) fn filter_available_versions<'a>(
    versions: &'a [RemoteVersion],
    query: &str,
    limit: usize,
    active_filters: &HashSet<SearchFilter>,
    installed_set: &HashSet<String>,
    schedule: Option<&ReleaseSchedule>,
) -> Vec<&'a RemoteVersion> {
    let query_lower = query.to_lowercase();

    if let Some(resolved) = resolve_alias(versions, query) {
        return vec![resolved];
    }

    let mut result = if query_lower == "lts" {
        let mut filtered: Vec<&RemoteVersion> = versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .collect();
        filtered.sort_by(|a, b| b.version.cmp(&a.version));

        let mut latest_by_major: HashMap<u32, &RemoteVersion> = HashMap::new();
        for v in &filtered {
            latest_by_major
                .entry(v.version.major)
                .and_modify(|existing| {
                    if v.version > existing.version {
                        *existing = v;
                    }
                })
                .or_insert(v);
        }

        let mut r: Vec<&RemoteVersion> = latest_by_major.into_values().collect();
        r.sort_by(|a, b| b.version.cmp(&a.version));
        r.truncate(limit);
        r
    } else {
        let mut filtered: Vec<&RemoteVersion> = versions
            .iter()
            .filter(|v| {
                let version_str = v.version.to_string();

                version_str.contains(query)
                    || v.lts_codename
                        .as_ref()
                        .is_some_and(|c| c.to_lowercase().contains(&query_lower))
            })
            .collect();

        filtered.sort_by(|a, b| b.version.cmp(&a.version));

        let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();
        for v in &filtered {
            let key = (v.version.major, v.version.minor);
            latest_by_minor
                .entry(key)
                .and_modify(|existing| {
                    if v.version.patch > existing.version.patch {
                        *existing = v;
                    }
                })
                .or_insert(v);
        }

        let mut r: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
        r.sort_by(|a, b| b.version.cmp(&a.version));
        r.truncate(limit);
        r
    };

    if !active_filters.is_empty() {
        result.retain(|v| {
            if active_filters.contains(&SearchFilter::Lts) && v.lts_codename.is_none() {
                return false;
            }
            let version_str = v.version.to_string();
            if active_filters.contains(&SearchFilter::Installed)
                && !installed_set.contains(&version_str)
            {
                return false;
            }
            if active_filters.contains(&SearchFilter::NotInstalled)
                && installed_set.contains(&version_str)
            {
                return false;
            }
            if active_filters.contains(&SearchFilter::Eol) {
                let is_eol = schedule.is_some_and(|s| !s.is_active(v.version.major));
                if !is_eol {
                    return false;
                }
            }
            if active_filters.contains(&SearchFilter::Active) {
                let is_active = schedule.is_none_or(|s| s.is_active(v.version.major));
                if !is_active {
                    return false;
                }
            }
            true
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{filter_available_versions, resolve_alias};
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
    fn resolve_alias_latest_returns_highest_version() {
        let versions = vec![remote("v20.11.0", None), remote("v22.1.0", Some("Jod"))];

        let resolved = resolve_alias(&versions, "latest").expect("latest alias should resolve");

        assert_eq!(resolved.version.to_string(), "v22.1.0");
    }

    #[test]
    fn resolve_alias_lts_codename_is_case_insensitive() {
        let versions = vec![
            remote("v20.11.0", Some("Iron")),
            remote("v20.12.0", Some("Iron")),
            remote("v22.1.0", Some("Jod")),
        ];

        let resolved = resolve_alias(&versions, "lts/iron").expect("lts codename should resolve");

        assert_eq!(resolved.version.to_string(), "v20.12.0");
    }

    #[test]
    fn non_lts_query_honors_limit_argument() {
        let versions = vec![
            remote("v22.3.0", None),
            remote("v22.2.0", None),
            remote("v22.1.0", None),
        ];

        let filtered =
            filter_available_versions(&versions, "v22", 2, &HashSet::new(), &HashSet::new(), None);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].version.to_string(), "v22.3.0");
        assert_eq!(filtered[1].version.to_string(), "v22.2.0");
    }

    #[test]
    fn active_filters_installed_and_eol_are_applied() {
        let versions = vec![
            remote("v22.1.0", Some("Jod")),
            remote("v20.11.0", Some("Iron")),
        ];
        let installed = HashSet::from(["v20.11.0".to_string()]);
        let filters = HashSet::from([SearchFilter::Installed, SearchFilter::Eol]);
        let schedule = schedule_with_eol_major(20);

        let filtered =
            filter_available_versions(&versions, "v", 10, &filters, &installed, Some(&schedule));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].version.to_string(), "v20.11.0");
    }
}
