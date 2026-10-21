use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};

use super::Track;

pub(super) type Sources = HashMap<PathBuf, PathBuf>;
pub(super) type Tracks = HashMap<PathBuf, Track>;

pub(super) struct Store(Connection);

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        fs::create_dir_all(path.parent().context("Invalid library database path")?)?;
        let connection = Connection::open(path).context("Cannot open library database")?;
        connection.busy_timeout(Duration::from_secs(3))?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => connection.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE tracks (path BLOB PRIMARY KEY NOT NULL, title TEXT NOT NULL);
                 CREATE TABLE sources (path BLOB PRIMARY KEY NOT NULL, canonical_path BLOB NOT NULL);
                 PRAGMA user_version = 1;
                 COMMIT;"
            ).context("Cannot initialize library database")?,
            1 | 2 => {},
            _ => bail!("Library database version {version} is newer than this application supports"),
        }
        if version < 2 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                 ALTER TABLE tracks ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';
                 PRAGMA user_version = 2;
                 COMMIT;",
                )
                .context("Cannot migrate library metadata schema")?;
        }
        Ok(Self(connection))
    }

    pub fn load(&self) -> Result<(Sources, Tracks)> {
        let mut statement = self.0.prepare("SELECT path, canonical_path FROM sources")?;
        let sources = statement
            .query_map([], |row| {
                Ok((decode_path(row.get(0)?), decode_path(row.get(1)?)))
            })?
            .collect::<rusqlite::Result<Sources>>()?;
        let mut statement = self
            .0
            .prepare("SELECT path, title, metadata_json FROM tracks")?;
        let tracks = statement
            .query_map([], |row| {
                let path = decode_path(row.get(0)?);
                Ok((
                    path.clone(),
                    Track {
                        path,
                        title: row.get(1)?,
                        metadata: serde_json::from_str(&row.get::<_, String>(2)?).map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    2,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            },
                        )?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<Tracks>>()?;
        Ok((sources, tracks))
    }

    /// Commit a coherent snapshot. Unchanged tracks are not rewritten.
    pub fn save(&mut self, sources: &Sources, tracks: &Tracks) -> Result<()> {
        let (_, previous) = self.load()?;
        let tx = self.0.transaction()?;
        for path in previous.keys().filter(|path| !tracks.contains_key(*path)) {
            tx.execute("DELETE FROM tracks WHERE path = ?1", [encode_path(path)])?;
        }
        for track in tracks
            .values()
            .filter(|track| previous.get(&track.path) != Some(*track))
        {
            tx.execute(
                "INSERT INTO tracks(path, title, metadata_json) VALUES (?1, ?2, ?3)
                 ON CONFLICT(path) DO UPDATE SET title = excluded.title, metadata_json = excluded.metadata_json",
                params![encode_path(&track.path), track.title, serde_json::to_string(&track.metadata)?],
            )?;
        }
        tx.execute("DELETE FROM sources", [])?;
        for (path, canonical) in sources {
            tx.execute(
                "INSERT INTO sources VALUES (?1, ?2)",
                params![encode_path(path), encode_path(canonical)],
            )?;
        }
        tx.commit().context("Cannot save library database")
    }
}

// Preserve native filenames without lossy Unicode conversion. A local database
// belongs to its OS; it is not a portable playlist/export format.
#[cfg(windows)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
fn decode_path(bytes: Vec<u8>) -> PathBuf {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    PathBuf::from(OsString::from_wide(
        &bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
    ))
}

#[cfg(unix)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(unix)]
fn decode_path(bytes: Vec<u8>) -> PathBuf {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    PathBuf::from(OsString::from_vec(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_tracks_and_sources_and_reconciles_deleted_files() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("data/library.sqlite3");
        let path = temp.path().join("Music with spaces/Музика/song.mp3");
        let sources = Sources::from([(temp.path().into(), temp.path().into())]);
        let tracks = Tracks::from([(
            path.clone(),
            Track {
                path,
                title: "song".into(),
                ..Track::default()
            },
        )]);
        Store::open(&db).unwrap().save(&sources, &tracks).unwrap();
        let mut store = Store::open(&db).unwrap();
        assert_eq!(store.load().unwrap(), (sources.clone(), tracks));
        store.save(&sources, &Tracks::new()).unwrap();
        assert!(Store::open(&db).unwrap().load().unwrap().1.is_empty());
        store.save(&Sources::new(), &Tracks::new()).unwrap();
        assert!(store.load().unwrap().0.is_empty());
    }

    #[test]
    fn failed_transaction_keeps_previous_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("library.sqlite3")).unwrap();
        let track = Track {
            path: "song.mp3".into(),
            title: "song".into(),
            ..Track::default()
        };
        let tracks = Tracks::from([(track.path.clone(), track)]);
        store.save(&Sources::new(), &tracks).unwrap();
        store.0.execute_batch("CREATE TRIGGER fail_delete BEFORE DELETE ON tracks BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
        assert!(store.save(&Sources::new(), &Tracks::new()).is_err());
        assert_eq!(store.load().unwrap().1, tracks);
    }

    #[test]
    fn does_not_replace_invalid_or_newer_databases() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("library.sqlite3");
        fs::write(&db, b"invalid database").unwrap();
        assert!(Store::open(&db).is_err());
        assert_eq!(fs::read(&db).unwrap(), b"invalid database");
        fs::remove_file(&db).unwrap();
        Connection::open(&db)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        assert!(Store::open(&db).is_err());
        assert_eq!(
            Connection::open(db)
                .unwrap()
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            99
        );
    }

    #[test]
    fn migrates_version_one_without_losing_tracks_and_persists_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.sqlite3");
        let old = Connection::open(&path).unwrap();
        old.execute_batch(
            "CREATE TABLE tracks (path BLOB PRIMARY KEY NOT NULL, title TEXT NOT NULL);
            CREATE TABLE sources (path BLOB PRIMARY KEY NOT NULL, canonical_path BLOB NOT NULL);
            PRAGMA user_version = 1;",
        )
        .unwrap();
        old.execute(
            "INSERT INTO tracks VALUES (?1, 'Old title')",
            [encode_path(Path::new("music.wav"))],
        )
        .unwrap();
        drop(old);
        let mut store = Store::open(&path).unwrap();
        let (_, mut tracks) = store.load().unwrap();
        let track = tracks.values_mut().next().unwrap();
        assert_eq!(track.title, "Old title");
        assert_eq!(track.metadata.version, 0);
        track.metadata.artists = vec!["Artist A".into(), "Artist B".into()];
        track.metadata.release_date = Some("2025-05-02".into());
        track.metadata.duration_ms = Some(123456);
        store.save(&Sources::new(), &tracks).unwrap();
        assert_eq!(Store::open(&path).unwrap().load().unwrap().1, tracks);
        assert_eq!(
            store
                .0
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
    }
}
