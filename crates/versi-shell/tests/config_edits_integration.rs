use tempfile::tempdir;
use versi_backend::ShellInitOptions;
use versi_shell::{ShellConfig, ShellType};

#[test]
fn add_init_then_update_flags_persists_to_disk() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join(".bashrc");
    std::fs::write(&config_path, "export PATH=$PATH:/usr/local/bin\n").expect("write config");

    let mut config = ShellConfig::load(ShellType::Bash, config_path.clone()).expect("load config");
    let edit = config.add_init(r#"eval "$(fnm env --shell bash)""#, "fnm");
    assert!(edit.has_changes());
    config.apply_edit(&edit).expect("apply add-init edit");

    let mut config =
        ShellConfig::load(ShellType::Bash, config_path.clone()).expect("reload config");
    assert!(config.has_init("fnm env"));

    let options = ShellInitOptions {
        use_on_cd: true,
        resolve_engines: true,
        corepack_enabled: false,
    };
    let update = config.update_flags("fnm env", &options);
    assert!(update.has_changes());
    config.apply_edit(&update).expect("apply flags edit");

    let content = std::fs::read_to_string(&config_path).expect("read updated config");
    assert!(content.contains("--use-on-cd"));
    assert!(content.contains("--resolve-engines"));
    assert!(!content.contains("--corepack-enabled"));
}

#[test]
fn update_flags_removes_disabled_flags_without_touching_other_content() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join(".zshrc");
    std::fs::write(
        &config_path,
        "# keep me\n\
         eval \"$(fnm env --use-on-cd --resolve-engines --corepack-enabled --shell zsh)\"\n",
    )
    .expect("write config");

    let mut config = ShellConfig::load(ShellType::Zsh, config_path.clone()).expect("load config");
    let options = ShellInitOptions {
        use_on_cd: false,
        resolve_engines: true,
        corepack_enabled: false,
    };
    let update = config.update_flags("fnm env", &options);
    assert!(update.has_changes());
    config.apply_edit(&update).expect("apply flags edit");

    let content = std::fs::read_to_string(&config_path).expect("read updated config");
    assert!(content.contains("# keep me"));
    assert!(!content.contains("--use-on-cd"));
    assert!(content.contains("--resolve-engines"));
    assert!(!content.contains("--corepack-enabled"));
}
