use std::collections::HashMap;
use std::sync::{OnceLock, mpsc};
use std::time::Duration;

use chrono::Utc;
use versi_backend::RemoteVersion;
use versi_core::{ReleaseSchedule, VersionMeta};

const CACHE_SAVE_DEBOUNCE: Duration = Duration::from_millis(250);
const CACHE_SAVE_QUEUE_CAPACITY: usize = 16;

enum CacheSaveMessage {
    RemoteVersions(Vec<RemoteVersion>),
    ReleaseSchedule(ReleaseSchedule),
    VersionMetadata(HashMap<String, VersionMeta>),
}

#[derive(Default)]
struct CacheSnapshot {
    remote_versions: Vec<RemoteVersion>,
    release_schedule: Option<ReleaseSchedule>,
    version_metadata: Option<HashMap<String, VersionMeta>>,
}

impl CacheSnapshot {
    fn from_disk_cache(cache: crate::cache::DiskCache) -> Self {
        Self {
            remote_versions: cache.remote_versions,
            release_schedule: cache.release_schedule,
            version_metadata: cache.version_metadata,
        }
    }

    fn apply_message(&mut self, message: CacheSaveMessage) {
        match message {
            CacheSaveMessage::RemoteVersions(versions) => self.remote_versions = versions,
            CacheSaveMessage::ReleaseSchedule(schedule) => self.release_schedule = Some(schedule),
            CacheSaveMessage::VersionMetadata(metadata) => self.version_metadata = Some(metadata),
        }
    }

    fn persist(&self) {
        crate::cache::save_snapshot(
            &self.remote_versions,
            self.release_schedule.as_ref(),
            self.version_metadata.as_ref(),
            Utc::now(),
        );
    }
}

pub(super) fn enqueue_cache_save_remote_versions(versions: Vec<RemoteVersion>) {
    enqueue_cache_save(CacheSaveMessage::RemoteVersions(versions));
}

pub(super) fn enqueue_cache_save_release_schedule(schedule: ReleaseSchedule) {
    enqueue_cache_save(CacheSaveMessage::ReleaseSchedule(schedule));
}

pub(super) fn enqueue_cache_save_version_metadata(metadata: HashMap<String, VersionMeta>) {
    enqueue_cache_save(CacheSaveMessage::VersionMetadata(metadata));
}

fn enqueue_cache_save(message: CacheSaveMessage) {
    let sender = cache_save_sender();
    match sender.try_send(message) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(message)) => {
            if sender.send(message).is_err() {
                log::debug!("Cache save worker disconnected; dropping cache update");
            }
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            log::debug!("Cache save worker disconnected; dropping cache update");
        }
    }
}

fn cache_save_sender() -> &'static mpsc::SyncSender<CacheSaveMessage> {
    static CACHE_SAVER: OnceLock<mpsc::SyncSender<CacheSaveMessage>> = OnceLock::new();

    CACHE_SAVER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<CacheSaveMessage>(CACHE_SAVE_QUEUE_CAPACITY);
        std::thread::spawn(move || {
            let mut snapshot = match crate::cache::DiskCache::load() {
                Ok(Some(cache)) => CacheSnapshot::from_disk_cache(cache),
                Ok(None) => CacheSnapshot::default(),
                Err(error) => {
                    log::debug!("Failed to seed cache save worker from disk cache: {error}");
                    CacheSnapshot::default()
                }
            };

            while let Ok(message) = receiver.recv() {
                snapshot.apply_message(message);
                loop {
                    match receiver.recv_timeout(CACHE_SAVE_DEBOUNCE) {
                        Ok(next) => snapshot.apply_message(next),
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            snapshot.persist();
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            snapshot.persist();
                            return;
                        }
                    }
                }
            }
        });
        sender
    })
}
