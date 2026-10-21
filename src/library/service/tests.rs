use super::*;

fn track(path: PathBuf) -> Track {
    Track {
        title: path.file_stem().unwrap().to_string_lossy().into(),
        path,
        ..Track::default()
    }
}

#[test]
fn reconciliation_preserves_failed_subtrees_and_removes_confirmed_deletions() {
    let root = PathBuf::from("music");
    let offline = root.join("offline");
    let old = track(offline.join("saved.mp3"));
    let removed = track(root.join("deleted.mp3"));
    let new = track(root.join("new.mp3"));
    let mut tracks = Tracks::from([
        (old.path.clone(), old.clone()),
        (removed.path.clone(), removed.clone()),
    ]);
    reconcile(
        &mut tracks,
        &root,
        ScanResult {
            tracks: vec![new.clone()],
            errors: 1,
            failed_paths: vec![offline],
        },
    );
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks.get(&old.path), Some(&old));
    assert_eq!(tracks.get(&new.path), Some(&new));
    assert!(!tracks.contains_key(&removed.path));
    retain_sources(&mut tracks, &Sources::new());
    assert!(tracks.is_empty());
}

#[test]
fn offline_sources_use_persisted_canonical_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("missing");
    let canonical = temp.path().join("previous-canonical");
    let previous = Sources::from([(source.clone(), canonical.clone())]);
    assert_eq!(
        resolve_sources(std::slice::from_ref(&source), &previous).get(&source),
        Some(&canonical)
    );
}

#[test]
fn scoped_events_cover_rename_paths_and_ignore_unrelated_or_access_events() {
    use notify::event::{AccessKind, ModifyKind, RenameMode};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("music");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let sources = Sources::from([(root.clone(), root.clone())]);
    let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path(root.join("old.mp3"))
        .add_path(root.join("new.mp3"));
    assert_eq!(
        compact_scopes(event_scopes(&rename, &sources)),
        vec![root.clone()]
    );
    let unrelated = Event::new(EventKind::Any).add_path(temp.path().join("other.mp3"));
    assert!(event_scopes(&unrelated, &sources).is_empty());
    let access = Event::new(EventKind::Access(AccessKind::Any)).add_path(root.join("song.mp3"));
    assert!(event_scopes(&access, &sources).is_empty());
}

fn wait_for(
    updates: &async_channel::Receiver<Update>,
    predicate: impl Fn(&Snapshot) -> bool,
) -> Snapshot {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        match updates.try_recv() {
            Ok(Update::Snapshot(snapshot)) if predicate(&snapshot) => return snapshot,
            Ok(Update::Failed { message, .. }) => panic!("{message}"),
            _ => thread::sleep(Duration::from_millis(20)),
        }
    }
    panic!("Timed out waiting for library update");
}

#[test]
fn native_watcher_updates_database_for_create_rename_delete_and_source_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("music");
    std::fs::create_dir(&root).unwrap();
    let db = temp.path().join("data/library.sqlite3");
    let (service, updates) = LibraryService::start(db.clone(), vec![root.clone()]).unwrap();
    wait_for(&updates, |s| !s.scanning && s.tracks.is_empty());
    std::fs::write(root.join("first.mp3"), []).unwrap();
    wait_for(&updates, |s| {
        s.tracks.len() == 1 && s.tracks[0].title == "first"
    });
    std::fs::rename(root.join("first.mp3"), root.join("renamed.FLAC")).unwrap();
    wait_for(&updates, |s| {
        s.tracks.len() == 1 && s.tracks[0].title == "renamed"
    });
    assert_eq!(
        Store::open(&db)
            .unwrap()
            .load()
            .unwrap()
            .1
            .values()
            .next()
            .unwrap()
            .title,
        "renamed"
    );
    std::fs::remove_file(root.join("renamed.FLAC")).unwrap();
    wait_for(&updates, |s| !s.scanning && s.tracks.is_empty());
    std::fs::write(root.join("retained.ogg"), []).unwrap();
    wait_for(&updates, |s| s.tracks.len() == 1);
    service.set_sources(1, Vec::new()).unwrap();
    wait_for(&updates, |s| {
        s.revision == 1 && !s.scanning && s.tracks.is_empty()
    });
    assert!(Store::open(&db).unwrap().load().unwrap().1.is_empty());
    drop(service);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !updates.is_closed() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(updates.is_closed(), "Library worker did not stop");
}

#[test]
fn cached_tracks_are_published_before_startup_scan_and_survive_offline_sources() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("offline");
    let song = track(source.join("cached.mp3"));
    let db = temp.path().join("data/library.sqlite3");
    let sources = Sources::from([(source.clone(), source.clone())]);
    Store::open(&db)
        .unwrap()
        .save(&sources, &Tracks::from([(song.path.clone(), song)]))
        .unwrap();
    let (service, updates) = LibraryService::start(db.clone(), vec![source]).unwrap();
    wait_for(&updates, |s| s.scanning && s.tracks.len() == 1);
    wait_for(&updates, |s| {
        !s.scanning && s.tracks.len() == 1 && s.errors == 1
    });
    assert_eq!(Store::open(&db).unwrap().load().unwrap().1.len(), 1);
    drop(service);
}

#[test]
fn watcher_handles_nested_directory_moves_and_recreated_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("music");
    std::fs::create_dir(&root).unwrap();
    let (service, updates) =
        LibraryService::start(temp.path().join("data/library.sqlite3"), vec![root.clone()])
            .unwrap();
    wait_for(&updates, |s| !s.scanning);
    let album = root.join("album");
    std::fs::create_dir(&album).unwrap();
    std::fs::write(album.join("song.mp3"), []).unwrap();
    wait_for(&updates, |s| s.tracks.len() == 1);
    // Moving the whole source outside the configured location removes its rows.
    std::fs::rename(&root, temp.path().join("moved-away")).unwrap();
    wait_for(&updates, |s| !s.scanning && s.tracks.is_empty());
    std::fs::create_dir_all(root.join("new/album")).unwrap();
    std::fs::write(root.join("new/album/returned.ogg"), []).unwrap();
    wait_for(&updates, |s| {
        s.tracks.len() == 1 && s.tracks[0].title == "returned"
    });
    drop(service);
}

#[test]
fn watcher_refreshes_changed_tags_and_persists_metadata() {
    use lofty::{
        prelude::{Accessor, TagExt},
        tag::{Tag, TagType},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("music");
    std::fs::create_dir(&root).unwrap();
    let path = root.join("song.wav");
    crate::library::metadata::tests::wav(&path);
    let db = temp.path().join("data/library.sqlite3");
    let (service, updates) = LibraryService::start(db.clone(), vec![root]).unwrap();
    wait_for(&updates, |s| {
        !s.scanning && s.tracks.len() == 1 && s.tracks[0].metadata.duration_ms == Some(1000)
    });
    let mut tag = Tag::new(TagType::Id3v2);
    tag.set_title("Title from tags".into());
    tag.set_artist("Artist from tags".into());
    tag.save_to_path(&path, lofty::config::WriteOptions::default())
        .unwrap();
    wait_for(&updates, |s| {
        s.tracks.len() == 1 && s.tracks[0].title == "Title from tags"
    });
    let (_, cached) = Store::open(&db).unwrap().load().unwrap();
    assert_eq!(
        cached.values().next().unwrap().metadata.artists,
        ["Artist from tags"]
    );
    drop(service);
}
