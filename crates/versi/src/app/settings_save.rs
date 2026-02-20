use std::sync::{OnceLock, mpsc};
use std::time::Duration;

use crate::settings::AppSettings;

const SETTINGS_SAVE_DEBOUNCE: Duration = Duration::from_millis(250);

pub(super) fn enqueue_settings_save(settings: AppSettings) {
    let _ = settings_save_sender().send(settings);
}

fn settings_save_sender() -> &'static mpsc::Sender<AppSettings> {
    static SETTINGS_SAVER: OnceLock<mpsc::Sender<AppSettings>> = OnceLock::new();

    SETTINGS_SAVER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<AppSettings>();
        std::thread::spawn(move || {
            while let Ok(mut latest) = receiver.recv() {
                loop {
                    match receiver.recv_timeout(SETTINGS_SAVE_DEBOUNCE) {
                        Ok(next) => latest = next,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if let Err(error) = latest.save() {
                                log::error!("Failed to save settings: {error}");
                            }
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            if let Err(error) = latest.save() {
                                log::error!("Failed to save settings: {error}");
                            }
                            return;
                        }
                    }
                }
            }
        });
        sender
    })
}
