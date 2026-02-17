use std::sync::{OnceLock, mpsc};
use std::time::Duration;

pub(super) fn enqueue_cache_save(cache: crate::cache::DiskCache) {
    let _ = cache_save_sender().send(cache);
}

fn cache_save_sender() -> &'static mpsc::Sender<crate::cache::DiskCache> {
    static CACHE_SAVER: OnceLock<mpsc::Sender<crate::cache::DiskCache>> = OnceLock::new();

    CACHE_SAVER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<crate::cache::DiskCache>();
        std::thread::spawn(move || {
            let debounce_window = Duration::from_millis(250);
            while let Ok(mut latest) = receiver.recv() {
                loop {
                    match receiver.recv_timeout(debounce_window) {
                        Ok(next) => latest = next,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            latest.save();
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            latest.save();
                            return;
                        }
                    }
                }
            }
        });
        sender
    })
}
