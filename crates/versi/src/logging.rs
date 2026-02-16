#[cfg(debug_assertions)]
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use versi_platform::AppPaths;

struct ResilientFileWriter {
    path: PathBuf,
    file: Mutex<Option<File>>,
}

impl ResilientFileWriter {
    fn new(path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            file: Mutex::new(Some(file)),
        })
    }

    fn ensure_file(&self) -> io::Result<()> {
        let mut guard = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !self.path.exists() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *guard = Some(file);
        }

        Ok(())
    }
}

impl Write for ResilientFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_file()?;
        let mut guard = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut file) = *guard {
            file.write(buf)
        } else {
            Err(io::Error::other("File not available"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut file) = *guard {
            file.flush()
        } else {
            Ok(())
        }
    }
}

fn trim_log_file_if_oversized(log_path: &Path, max_log_size: u64) {
    if let Ok(metadata) = std::fs::metadata(log_path)
        && metadata.len() > max_log_size
        && let Ok(contents) = std::fs::read(log_path)
    {
        let half = contents.len() / 2;
        let keep_from = contents[half..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(half, |pos| half + pos + 1);
        let _ = std::fs::write(log_path, &contents[keep_from..]);
    }
}

pub fn init_logging(debug_enabled: bool, max_log_size: u64) {
    let Ok(paths) = AppPaths::new() else {
        return;
    };
    let _ = paths.ensure_dirs();
    let log_path = paths.log_file();

    trim_log_file_if_oversized(&log_path, max_log_size);

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .add_filter_allow_str("versi")
        .build();

    let file_logger = ResilientFileWriter::new(log_path.clone())
        .ok()
        .map(|writer| WriteLogger::new(LevelFilter::Debug, config.clone(), writer));

    #[cfg(debug_assertions)]
    {
        let term_logger = TermLogger::new(
            LevelFilter::Debug,
            config,
            TerminalMode::Mixed,
            ColorChoice::Auto,
        );

        if let Some(file_logger) = file_logger {
            let _ = CombinedLogger::init(vec![term_logger, file_logger]);
        } else {
            let _ = CombinedLogger::init(vec![term_logger]);
        }
    }

    #[cfg(not(debug_assertions))]
    {
        if let Some(file_logger) = file_logger {
            let _ = CombinedLogger::init(vec![file_logger]);
        }
    }

    set_logging_enabled(debug_enabled);

    if debug_enabled {
        log::info!(
            "Debug logging initialized, log file: {}",
            log_path.display()
        );
    }
}

pub fn set_logging_enabled(enabled: bool) {
    if enabled {
        log::set_max_level(log::LevelFilter::Debug);
    } else {
        log::set_max_level(log::LevelFilter::Off);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::{ResilientFileWriter, set_logging_enabled, trim_log_file_if_oversized};

    #[test]
    fn resilient_writer_recreates_missing_file_on_write() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let log_path = temp_dir.path().join("versi.log");
        let mut writer =
            ResilientFileWriter::new(log_path.clone()).expect("writer should open log file");

        writer
            .write_all(b"first line\n")
            .expect("initial write should succeed");
        std::fs::remove_file(&log_path).expect("log file should be removable");
        writer
            .write_all(b"second line\n")
            .expect("writer should recreate file after deletion");

        let contents =
            std::fs::read_to_string(&log_path).expect("recreated file should be readable");
        assert_eq!(contents, "second line\n");
    }

    #[test]
    fn trim_log_file_keeps_recent_half() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let log_path = temp_dir.path().join("debug.log");
        let original = "line-1\nline-2\nline-3\nline-4\nline-5\n";
        std::fs::write(&log_path, original).expect("test log file should be written");

        trim_log_file_if_oversized(&log_path, 10);

        let trimmed =
            std::fs::read_to_string(&log_path).expect("trimmed log file should be readable");
        assert!(trimmed.starts_with("line-4\n") || trimmed.starts_with("line-3\n"));
        assert!(!trimmed.contains("line-1"));
    }

    #[test]
    fn set_logging_enabled_updates_global_level() {
        set_logging_enabled(true);
        assert_eq!(log::max_level(), log::LevelFilter::Debug);

        set_logging_enabled(false);
        assert_eq!(log::max_level(), log::LevelFilter::Off);
    }
}
