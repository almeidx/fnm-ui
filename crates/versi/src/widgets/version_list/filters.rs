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
                        .map(|c| c.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
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
        r.truncate(20);
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
                let is_eol = schedule
                    .map(|s| !s.is_active(v.version.major))
                    .unwrap_or(false);
                if !is_eol {
                    return false;
                }
            }
            if active_filters.contains(&SearchFilter::Active) {
                let is_active = schedule
                    .map(|s| s.is_active(v.version.major))
                    .unwrap_or(true);
                if !is_active {
                    return false;
                }
            }
            true
        });
    }

    result
}
