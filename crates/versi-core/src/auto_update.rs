use std::io::Read;
use std::path::Path;

use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UpdateProgress {
    Downloading { downloaded: u64, total: u64 },
    Extracting,
    Applying,
    Complete(ApplyResult),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum ApplyResult {
    RestartRequired,
    ExitForInstaller,
}

#[derive(Debug, Error)]
pub enum AutoUpdateError {
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("{context}: {source}")]
    Http {
        context: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{context}: {source}")]
    Zip {
        context: &'static str,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("{context}: {details}")]
    Platform {
        context: &'static str,
        details: String,
    },
    #[error("{0}")]
    Invalid(String),
}

impl AutoUpdateError {
    fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    fn http(context: &'static str, source: reqwest::Error) -> Self {
        Self::Http { context, source }
    }

    fn zip(context: &'static str, source: zip::result::ZipError) -> Self {
        Self::Zip { context, source }
    }

    fn platform(context: &'static str, details: String) -> Self {
        Self::Platform { context, details }
    }

    fn io_with_path(context: &'static str, path: &Path, source: &std::io::Error) -> Self {
        Self::io(
            context,
            std::io::Error::new(source.kind(), format!("{}: {source}", path.display())),
        )
    }
}

/// Download and apply a packaged Versi update.
///
/// # Errors
/// Returns an error when downloading, extracting, or applying the update fails.
pub async fn download_and_apply(
    client: &reqwest::Client,
    download_url: &str,
    checksum_url: Option<&str>,
    progress: mpsc::Sender<UpdateProgress>,
) -> Result<ApplyResult, AutoUpdateError> {
    let cache_dir = versi_platform::AppPaths::new()
        .map_err(|error| AutoUpdateError::platform("failed to resolve app paths", error))?
        .cache_dir;
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| AutoUpdateError::io("failed to create cache directory", error))?;

    let temp_dir = tempfile::tempdir_in(&cache_dir)
        .map_err(|error| AutoUpdateError::io("failed to create temp directory", error))?;

    let raw_name = download_url.rsplit('/').next().unwrap_or("update-download");
    let file_name = Path::new(raw_name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && !n.contains(".."))
        .unwrap_or("update-download");
    let download_path = temp_dir.path().join(file_name);

    info!("Downloading update from {download_url}");
    download_file(client, download_url, &download_path, &progress).await?;
    verify_download_checksum(client, checksum_url, file_name, &download_path).await?;

    let is_msi = Path::new(file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"));

    if is_msi {
        let _ = progress.send(UpdateProgress::Applying).await;
        let _ = temp_dir.keep();
        return apply_msi(&download_path);
    }

    let _ = progress.send(UpdateProgress::Extracting).await;
    let extract_dir = temp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|error| AutoUpdateError::io("failed to create extraction directory", error))?;
    extract_zip(&download_path, &extract_dir)?;

    let _ = progress.send(UpdateProgress::Applying).await;
    apply_update(&extract_dir)
}

async fn verify_download_checksum(
    client: &reqwest::Client,
    checksum_url: Option<&str>,
    asset_name: &str,
    downloaded_path: &Path,
) -> Result<(), AutoUpdateError> {
    let checksum_url = checksum_url.ok_or_else(|| {
        AutoUpdateError::Invalid(format!(
            "Missing checksum asset for {asset_name}. Refusing to apply unverified update."
        ))
    })?;
    let response = client
        .get(checksum_url)
        .send()
        .await
        .map_err(|error| AutoUpdateError::http("failed to download checksums", error))?;
    if !response.status().is_success() {
        return Err(AutoUpdateError::Invalid(format!(
            "Failed to download checksums: HTTP {}",
            response.status()
        )));
    }

    let checksums = response
        .text()
        .await
        .map_err(|error| AutoUpdateError::http("failed to read checksums", error))?;
    let expected = parse_expected_checksum(&checksums, asset_name).ok_or_else(|| {
        AutoUpdateError::Invalid(format!(
            "No checksum entry found for update asset '{asset_name}' in checksums file"
        ))
    })?;
    let actual = sha256_file(downloaded_path)?;

    if actual.eq_ignore_ascii_case(&expected) {
        info!("Update checksum verified for {asset_name}");
        Ok(())
    } else {
        Err(AutoUpdateError::Invalid(format!(
            "Checksum mismatch for {asset_name}. Refusing to apply update."
        )))
    }
}

fn parse_expected_checksum(checksums: &str, asset_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts
            .next()?
            .trim_start_matches('*')
            .trim_start_matches("./");
        if name == asset_name {
            Some(hash.to_ascii_lowercase())
        } else {
            None
        }
    })
}

fn sha256_file(path: &Path) -> Result<String, AutoUpdateError> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        AutoUpdateError::io_with_path("failed to open file for checksum", path, &error)
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            AutoUpdateError::io_with_path("failed to read file for checksum", path, &error)
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress: &mpsc::Sender<UpdateProgress>,
) -> Result<(), AutoUpdateError> {
    use futures_util::StreamExt;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AutoUpdateError::http("download request failed", error))?;

    if !response.status().is_success() {
        return Err(AutoUpdateError::Invalid(format!(
            "Download failed with status {}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(dest).await.map_err(|error| {
        AutoUpdateError::io_with_path("failed to create download file", dest, &error)
    })?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AutoUpdateError::http("download stream error", error))?;
        file.write_all(&chunk).await.map_err(|error| {
            AutoUpdateError::io_with_path("failed to write download data", dest, &error)
        })?;
        downloaded += chunk.len() as u64;
        let _ = progress
            .send(UpdateProgress::Downloading { downloaded, total })
            .await;
    }

    file.flush().await.map_err(|error| {
        AutoUpdateError::io_with_path("failed to flush download file", dest, &error)
    })?;

    info!("Download complete: {downloaded} bytes");
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    let file = std::fs::File::open(zip_path).map_err(|error| {
        AutoUpdateError::io_with_path("failed to open zip file", zip_path, &error)
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AutoUpdateError::zip("failed to read zip archive", error))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|error| AutoUpdateError::zip("failed to read zip entry", error))?;
        let Some(name) = entry.enclosed_name() else {
            warn!("Skipping zip entry with unsafe path");
            continue;
        };
        let out_path = dest.join(name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|error| {
                AutoUpdateError::io_with_path(
                    "failed to create extraction directory",
                    &out_path,
                    &error,
                )
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    AutoUpdateError::io_with_path(
                        "failed to create extraction parent directory",
                        parent,
                        &error,
                    )
                })?;
            }
            let mut outfile = std::fs::File::create(&out_path).map_err(|error| {
                AutoUpdateError::io_with_path("failed to create extracted file", &out_path, &error)
            })?;
            std::io::copy(&mut entry, &mut outfile).map_err(|error| {
                AutoUpdateError::io_with_path("failed to extract archive entry", &out_path, &error)
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ =
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    debug!("Extraction complete to {}", dest.display());
    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_update(extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    let new_app = find_app_bundle(extract_dir)?;
    let current_bundle = current_app_bundle()?;
    let old_bundle = current_bundle.with_extension("app.old");

    info!(
        "Replacing {} with {}",
        current_bundle.display(),
        new_app.display()
    );

    if old_bundle.exists() {
        std::fs::remove_dir_all(&old_bundle).map_err(|error| {
            AutoUpdateError::io_with_path("failed to remove old backup", &old_bundle, &error)
        })?;
    }

    std::fs::rename(&current_bundle, &old_bundle).map_err(|error| {
        AutoUpdateError::io_with_path(
            "failed to move current app bundle aside",
            &current_bundle,
            &error,
        )
    })?;

    match move_dir(&new_app, &current_bundle) {
        Ok(()) => {}
        Err(e) => {
            warn!("Apply failed, restoring backup: {e}");
            let _ = std::fs::rename(&old_bundle, &current_bundle);
            return Err(e);
        }
    }

    let _ = std::process::Command::new("xattr")
        .args(["-cr", &current_bundle.to_string_lossy()])
        .output();

    info!("macOS update applied successfully");
    Ok(ApplyResult::RestartRequired)
}

#[cfg(target_os = "macos")]
fn find_app_bundle(dir: &Path) -> Result<std::path::PathBuf, AutoUpdateError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|error| AutoUpdateError::io_with_path("failed to read extract dir", dir, &error))?
    {
        let entry = entry.map_err(|error| {
            AutoUpdateError::io("failed to read extract directory entry", error)
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") && path.is_dir() {
            return Ok(path);
        }
    }
    Err(AutoUpdateError::Invalid(
        "No .app bundle found in extracted archive".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Result<std::path::PathBuf, AutoUpdateError> {
    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;
    let mut path = exe.as_path();
    loop {
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            return Ok(path.to_path_buf());
        }
        path = path.parent().ok_or_else(|| {
            AutoUpdateError::Invalid("Current executable is not inside a .app bundle".to_string())
        })?;
    }
}

#[cfg(target_os = "macos")]
fn move_dir(src: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }

    copy_dir_recursive(src, dest)?;
    std::fs::remove_dir_all(src).map_err(|error| {
        AutoUpdateError::io_with_path("failed to clean up source directory", src, &error)
    })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    std::fs::create_dir_all(dest).map_err(|error| {
        AutoUpdateError::io_with_path("failed to create directory", dest, &error)
    })?;

    for entry in std::fs::read_dir(src)
        .map_err(|error| AutoUpdateError::io_with_path("failed to read directory", src, &error))?
    {
        let entry =
            entry.map_err(|error| AutoUpdateError::io("failed to read directory entry", error))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|error| {
                AutoUpdateError::io(
                    "failed to copy file during update apply",
                    std::io::Error::new(
                        error.kind(),
                        format!("{} -> {}: {error}", src_path.display(), dest_path.display()),
                    ),
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_update(extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    let new_binary = extract_dir.join("versi");
    if !new_binary.exists() {
        return Err(AutoUpdateError::Invalid(
            "No 'versi' binary found in extracted archive".to_string(),
        ));
    }

    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;

    info!("Replacing binary via self-replace");
    match self_replace::self_replace(&new_binary) {
        Ok(()) => {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
            info!("Linux update applied successfully");
            Ok(ApplyResult::RestartRequired)
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            info!("Permission denied, trying pkexec for elevated replacement");
            apply_update_with_pkexec(&new_binary, &exe)
        }
        Err(error) => Err(AutoUpdateError::io("failed to replace binary", error)),
    }
}

#[cfg(target_os = "linux")]
fn apply_update_with_pkexec(
    new_binary: &Path,
    target: &Path,
) -> Result<ApplyResult, AutoUpdateError> {
    let status = std::process::Command::new("pkexec")
        .args([
            "cp",
            "--",
            &new_binary.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .status()
        .map_err(|error| AutoUpdateError::io("failed to run pkexec", error))?;

    if !status.success() {
        return Err(AutoUpdateError::Invalid(format!(
            "Elevated update failed. Binary is installed in a system location.\n\
             To update manually, run:\n  sudo cp {} {}",
            new_binary.display(),
            target.display()
        )));
    }

    let _ = std::process::Command::new("pkexec")
        .args(["chmod", "755", &target.to_string_lossy()])
        .status();

    info!("Linux update applied via pkexec");
    Ok(ApplyResult::RestartRequired)
}

#[cfg(target_os = "windows")]
fn apply_update(_extract_dir: &Path) -> Result<ApplyResult, AutoUpdateError> {
    unreachable!("Windows uses MSI path, not extract+apply")
}

#[cfg(target_os = "windows")]
fn apply_msi(msi_path: &Path) -> Result<ApplyResult, AutoUpdateError> {
    info!("Launching MSI installer: {}", msi_path.display());
    std::process::Command::new("msiexec")
        .args(["/i", &msi_path.to_string_lossy(), "/passive"])
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to launch MSI installer", error))?;

    Ok(ApplyResult::ExitForInstaller)
}

#[cfg(not(target_os = "windows"))]
fn apply_msi(_msi_path: &Path) -> Result<ApplyResult, AutoUpdateError> {
    Err(AutoUpdateError::Invalid(
        "MSI installation is only supported on Windows".to_string(),
    ))
}

pub fn cleanup_old_app_bundle() {
    #[cfg(target_os = "macos")]
    {
        if let Ok(bundle) = current_app_bundle() {
            let old = bundle.with_extension("app.old");
            if old.exists() {
                info!("Cleaning up old app bundle: {}", old.display());
                let _ = std::fs::remove_dir_all(&old);
            }
        }
    }

    let Ok(paths) = versi_platform::AppPaths::new() else {
        return;
    };
    let cache_dir = paths.cache_dir;
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && entry.file_name().to_string_lossy().starts_with(".tmp") {
                debug!("Cleaning up update temp dir: {}", path.display());
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
}

#[cfg(target_os = "macos")]
/// Restart the current application bundle.
///
/// # Errors
/// Returns an error if the running app bundle cannot be located or reopened.
pub fn restart_app() -> Result<(), AutoUpdateError> {
    let bundle = current_app_bundle()?;
    std::process::Command::new("open")
        .args(["-n", &bundle.to_string_lossy()])
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to restart app", error))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
/// Restart the current executable.
///
/// # Errors
/// Returns an error if the current executable path cannot be resolved or a new
/// process cannot be spawned.
pub fn restart_app() -> Result<(), AutoUpdateError> {
    let exe = std::env::current_exe()
        .map_err(|error| AutoUpdateError::io("failed to get current executable", error))?;

    // On Linux, after self_replace, /proc/self/exe points to the old deleted inode
    // and current_exe() returns a path with " (deleted)" appended.
    // Strip it to get the actual path where the new binary was placed.
    #[cfg(target_os = "linux")]
    let exe = {
        let path_str = exe.to_string_lossy();
        if path_str.ends_with(" (deleted)") {
            let fixed = std::path::PathBuf::from(path_str.trim_end_matches(" (deleted)"));
            info!("Adjusted exe path from deleted inode: {}", fixed.display());
            fixed
        } else {
            exe
        }
    };

    info!("Restarting from: {}", exe.display());
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|error| AutoUpdateError::io("failed to restart app", error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::{AutoUpdateError, extract_zip, parse_expected_checksum, sha256_file};

    #[test]
    fn extract_zip_expands_files_and_directories() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let zip_path = temp.path().join("update.zip");
        let extract_dir = temp.path().join("extract");

        let zip_file = std::fs::File::create(&zip_path).expect("zip file should be created");
        let mut writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        writer
            .add_directory("nested/", options)
            .expect("directory entry should be written");
        writer
            .start_file("nested/versi", options)
            .expect("file entry should be started");
        writer
            .write_all(b"binary-content")
            .expect("file entry should be written");
        writer.finish().expect("zip archive should be finalized");

        extract_zip(&zip_path, &extract_dir).expect("zip should extract");

        let extracted = std::fs::read(extract_dir.join("nested/versi"))
            .expect("extracted file should exist and be readable");
        assert_eq!(extracted, b"binary-content");
    }

    #[test]
    fn extract_zip_skips_unsafe_paths() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let zip_path = temp.path().join("unsafe.zip");
        let extract_dir = temp.path().join("extract");

        let zip_file = std::fs::File::create(&zip_path).expect("zip file should be created");
        let mut writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        writer
            .start_file("../outside.txt", options)
            .expect("unsafe file entry should be started");
        writer
            .write_all(b"should not be extracted")
            .expect("unsafe file entry should be written");
        writer.finish().expect("zip archive should be finalized");

        extract_zip(&zip_path, &extract_dir).expect("zip extraction should not fail");

        assert!(
            !temp.path().join("outside.txt").exists(),
            "unsafe path should not be extracted outside destination"
        );
        assert!(
            !extract_dir.join("../outside.txt").exists(),
            "unsafe relative extraction output should not exist"
        );
    }

    #[test]
    fn parse_expected_checksum_matches_asset_name() {
        let checksums = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  foo.zip
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  bar.zip
";
        let parsed = parse_expected_checksum(checksums, "bar.zip");
        assert_eq!(
            parsed.as_deref(),
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
    }

    #[test]
    fn sha256_file_returns_known_digest() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let file_path = temp.path().join("payload.bin");
        std::fs::write(&file_path, b"versi").expect("payload file should be written");

        let digest = sha256_file(&file_path).expect("checksum should be computed");
        assert_eq!(
            digest,
            "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn apply_msi_reports_unsupported_on_non_windows() {
        let result = super::apply_msi(std::path::Path::new("/tmp/update.msi"));
        assert!(matches!(
            result,
            Err(AutoUpdateError::Invalid(ref message))
                if message == "MSI installation is only supported on Windows"
        ));
    }
}
