use super::*;

fn paths(directory: &std::path::Path) -> AppPaths {
    AppPaths {
        config_path: directory.join("config/settings.json"),
        logs_path: directory.join("logs"),
        library_path: directory.join("library.sqlite3"),
    }
}

#[test]
fn rebranding_reuses_legacy_data_but_prefers_existing_new_storage() {
    let temp = tempfile::tempdir().unwrap();
    let current = paths(&temp.path().join("Sonvail"));
    let legacy = paths(&temp.path().join("Soundora"));
    assert_eq!(
        AppPaths::prefer_existing(current.clone(), legacy.clone())
            .unwrap()
            .config_path,
        current.config_path
    );
    fs::create_dir_all(legacy.library_path.parent().unwrap()).unwrap();
    fs::write(&legacy.library_path, b"existing database").unwrap();
    assert_eq!(
        AppPaths::prefer_existing(current.clone(), legacy.clone())
            .unwrap()
            .library_path,
        legacy.library_path
    );
    fs::create_dir_all(current.config_path.parent().unwrap()).unwrap();
    fs::write(
        &current.config_path,
        b"invalid config must still take precedence",
    )
    .unwrap();
    assert_eq!(
        AppPaths::prefer_existing(current.clone(), legacy.clone())
            .unwrap()
            .config_path,
        current.config_path
    );
    assert_eq!(
        fs::read(&legacy.library_path).unwrap(),
        b"existing database"
    );
}

#[test]
fn missing_config_is_created_and_changes_survive_reload() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    let mut store = ConfigStore::load(paths.clone()).unwrap();
    assert_eq!(store.snapshot(), &AppConfig::default());
    let draft = AppConfig {
        theme: ThemeMode::System,
        library_folders: vec![
            directory.path().join("Music with spaces/Музика"),
            directory.path().join("Offline drive"),
        ],
    };
    store.save(draft.clone()).unwrap();
    let loaded = ConfigStore::load(paths.clone()).unwrap();
    assert_eq!(loaded.snapshot(), &draft);
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(paths.config_path).unwrap()).unwrap();
    assert_eq!(json["theme"], "system");
    assert_eq!(json["libraryFolders"].as_array().unwrap().len(), 2);
}

#[test]
fn partial_config_uses_defaults_without_rewriting_file() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    fs::create_dir_all(paths.config_path.parent().unwrap()).unwrap();
    fs::write(&paths.config_path, br#"{"theme":"light"}"#).unwrap();
    let store = ConfigStore::load(paths.clone()).unwrap();
    assert_eq!(store.snapshot().theme, ThemeMode::Light);
    assert!(store.snapshot().library_folders.is_empty());
    assert_eq!(
        fs::read(&paths.config_path).unwrap(),
        br#"{"theme":"light"}"#
    );
}

#[test]
fn invalid_config_is_reported_without_destroying_original() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    fs::create_dir_all(paths.config_path.parent().unwrap()).unwrap();
    for bytes in [
        b"broken json".as_slice(),
        br#"{"theme":"unknown"}"#,
        br#"{"typo":true}"#,
    ] {
        fs::write(&paths.config_path, bytes).unwrap();
        assert!(ConfigStore::load(paths.clone()).is_err());
        assert_eq!(fs::read(&paths.config_path).unwrap(), bytes);
    }
}

#[test]
fn failed_save_keeps_last_snapshot_and_previous_file() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    let mut store = ConfigStore::load(paths.clone()).unwrap();
    let original = fs::read(&paths.config_path).unwrap();
    // A directory at the destination cannot be replaced with the JSON file.
    store.paths.config_path = directory.path().join("occupied");
    fs::create_dir(&store.paths.config_path).unwrap();
    let draft = AppConfig {
        theme: ThemeMode::Light,
        ..AppConfig::default()
    };
    assert!(store.save(draft).is_err());
    assert_eq!(store.snapshot(), &AppConfig::default());
    assert_eq!(fs::read(&paths.config_path).unwrap(), original);
}

#[test]
fn initial_creation_does_not_clobber_existing_file() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    fs::create_dir_all(paths.config_path.parent().unwrap()).unwrap();
    fs::write(&paths.config_path, b"original").unwrap();
    assert!(ConfigStore::write(&paths, &AppConfig::default(), false).is_err());
    assert_eq!(fs::read(&paths.config_path).unwrap(), b"original");
}
