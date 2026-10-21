use super::*;

#[test]
fn scans_nested_audio_case_insensitively_and_sorts_by_filename() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("Music with spaces/Музика");
    fs::create_dir_all(&nested).unwrap();
    for name in [
        "Zulu.MP3",
        "alpha.wav",
        "Bravo.FlAc",
        "Голос.ogg",
        "cover.png",
        "notes.txt",
        "no-extension",
    ] {
        fs::write(nested.join(name), []).unwrap();
    }
    fs::create_dir(nested.join("directory.mp3")).unwrap();
    let result = scan(&[root.path().into()]);
    assert_eq!(result.errors, 0);
    assert_eq!(
        result
            .tracks
            .iter()
            .map(|track| track.title.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "Bravo", "Zulu", "Голос"]
    );
    assert!(
        result
            .tracks
            .iter()
            .all(|track| track.path.is_absolute() && track.path.is_file())
    );
}

#[test]
fn overlapping_and_repeated_sources_do_not_duplicate_tracks() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("album");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("song.mp3"), []).unwrap();
    let result = scan(&[
        root.path().into(),
        nested.clone(),
        nested.join(".."),
        nested,
    ]);
    assert_eq!(result.tracks.len(), 1);
    assert_eq!(result.errors, 0);
}

#[test]
fn unavailable_or_invalid_sources_do_not_stop_other_sources() {
    let root = tempfile::tempdir().unwrap();
    let track = root.path().join("song.mp3");
    fs::write(&track, []).unwrap();
    let result = scan(&[root.path().join("missing"), track, root.path().into()]);
    assert_eq!(result.errors, 2);
    assert_eq!(result.tracks.len(), 1);
}

#[test]
fn rescanning_reflects_deleted_files_and_removed_sources() {
    let root = tempfile::tempdir().unwrap();
    let track = root.path().join("song.mp3");
    fs::write(&track, []).unwrap();
    assert_eq!(scan(&[root.path().into()]).tracks.len(), 1);
    fs::remove_file(track).unwrap();
    assert!(scan(&[root.path().into()]).tracks.is_empty());
    assert!(scan(&[]).tracks.is_empty());
}

#[cfg(unix)]
#[test]
fn skips_symbolic_links_and_directory_cycles() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("song.mp3"), []).unwrap();
    std::os::unix::fs::symlink(root.path(), root.path().join("cycle")).unwrap();
    std::os::unix::fs::symlink(root.path().join("song.mp3"), root.path().join("alias.mp3"))
        .unwrap();
    let result = scan(&[root.path().into()]);
    assert_eq!(result.tracks.len(), 1);
    assert_eq!(result.errors, 0);
}
