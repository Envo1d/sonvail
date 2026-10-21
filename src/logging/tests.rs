use super::*;

#[test]
fn shutdown_flushes_pending_events_and_is_idempotent_across_clones() {
    let directory = tempfile::tempdir().unwrap();
    let (writer, guard) = file_writer(directory.path()).unwrap();
    let shutdown = ShutdownLogs::new(guard);
    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(writer),
    );
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(folder_count = 2, "settings persisted test event");
    });
    shutdown.clone().flush();
    shutdown.flush();
    let entries = fs_entries(directory.path());
    assert_eq!(entries.len(), 1);
    let name = entries[0].file_name().unwrap().to_string_lossy();
    assert!(name.starts_with("Sonvail."));
    assert!(name.ends_with(".log"));
    let content = std::fs::read_to_string(&entries[0]).unwrap();
    assert!(content.contains("settings persisted test event"));
    assert!(content.contains("folder_count=2"));
    assert!(!content.contains('\u{1b}'));
}

fn fs_entries(path: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect()
}

#[test]
fn writer_reports_unusable_directory() {
    let directory = tempfile::tempdir().unwrap();
    let occupied = directory.path().join("file");
    std::fs::write(&occupied, b"occupied").unwrap();
    assert!(file_writer(&occupied).is_err());
}
