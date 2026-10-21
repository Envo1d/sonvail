use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Context, Result, bail};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    prelude::Accessor,
    probe::Probe,
    tag::{ItemKey, Tag},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::Track;

const VERSION: u32 = 1;
const MAX_PICTURE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub version: u32,
    pub stamp: Option<FileStamp>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artists: Vec<String>,
    pub release_date: Option<String>,
    pub original_release_date: Option<String>,
    pub genres: Vec<String>,
    pub composers: Vec<String>,
    pub publisher: Option<String>,
    pub copyright: Option<String>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    pub duration_ms: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
    pub bitrate_kbps: Option<u32>,
    pub artwork: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStamp {
    pub size: u64,
    pub modified_ns: Option<u128>,
}

impl Metadata {
    pub fn artwork_path(&self, directory: &Path) -> Option<PathBuf> {
        self.artwork
            .as_ref()
            .filter(|key| key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|key| directory.join(format!("{key}.png")))
    }

    pub fn thumbnail_path(&self, directory: &Path) -> Option<PathBuf> {
        self.artwork_path(directory).map(|path| {
            path.with_file_name(format!(
                "{}-thumb.png",
                self.artwork.as_deref().unwrap_or_default()
            ))
        })
    }
}

pub(super) fn read_track(
    path: &Path,
    previous: Option<&Track>,
    artwork: Option<&Path>,
) -> std::io::Result<Track> {
    let stat = fs::metadata(path)?;
    let stamp = FileStamp {
        size: stat.len(),
        modified_ns: stat
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|time| time.as_nanos()),
    };
    if let Some(previous) = previous
        && previous.metadata.version == VERSION
        && previous.metadata.error.is_none()
        && previous.metadata.stamp.as_ref() == Some(&stamp)
        && stamp.modified_ns.is_some()
        && artwork.is_none_or(|dir| {
            previous.metadata.artwork_path(dir).is_none_or(|path| {
                path.is_file()
                    && previous
                        .metadata
                        .thumbnail_path(dir)
                        .is_some_and(|path| path.is_file())
            })
        })
    {
        return Ok(previous.clone());
    }
    let mut track = Track::from_path(path.to_owned());
    match extract(path, artwork) {
        Ok((title, metadata)) => {
            if let Some(title) = title {
                track.title = title;
            }
            track.metadata = metadata;
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %format!("{error:#}"), "Cannot read track metadata");
            if let Some(previous) = previous {
                track = previous.clone();
            }
            track.metadata.error = Some(format!("{error:#}"));
        }
    }
    track.metadata.version = VERSION;
    track.metadata.stamp = Some(stamp);
    Ok(track)
}

fn clean(value: &str) -> Option<String> {
    let value = value.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    (!value.is_empty()).then(|| value.to_owned())
}

fn values(tags: &[&Tag], keys: &[ItemKey]) -> Vec<String> {
    // Primary/richer tags win; do not add ID3v1's truncated duplicate artist.
    for tag in tags {
        let mut result = Vec::new();
        for key in keys {
            for value in tag
                .get_strings(*key)
                .flat_map(|value| value.split('\0'))
                .filter_map(clean)
            {
                if !result.contains(&value) {
                    result.push(value);
                }
            }
        }
        if !result.is_empty() {
            return result;
        }
    }
    Vec::new()
}

fn first(tags: &[&Tag], keys: &[ItemKey]) -> Option<String> {
    // Prefer the semantic key (e.g. release date) across all tag containers.
    keys.iter()
        .find_map(|key| values(tags, &[*key]).into_iter().next())
}

fn extract(path: &Path, artwork: Option<&Path>) -> Result<(Option<String>, Metadata)> {
    let file = Probe::open(path)?
        .guess_file_type()?
        .read()
        .context("Cannot parse audio tags/properties")?;
    let mut tags = Vec::new();
    if let Some(primary) = file.primary_tag() {
        tags.push(primary);
    }
    for tag in file.tags() {
        if !tags.iter().any(|existing| std::ptr::eq(*existing, tag)) {
            tags.push(tag);
        }
    }
    let props = file.properties();
    let mut metadata = Metadata {
        artists: values(&tags, &[ItemKey::TrackArtists, ItemKey::TrackArtist]),
        album: first(&tags, &[ItemKey::AlbumTitle]),
        album_artists: values(&tags, &[ItemKey::AlbumArtists, ItemKey::AlbumArtist]),
        release_date: first(
            &tags,
            &[ItemKey::ReleaseDate, ItemKey::RecordingDate, ItemKey::Year],
        ),
        original_release_date: first(&tags, &[ItemKey::OriginalReleaseDate]),
        genres: values(&tags, &[ItemKey::Genre]),
        composers: values(&tags, &[ItemKey::Composer]),
        publisher: first(&tags, &[ItemKey::Publisher, ItemKey::Label]),
        copyright: first(&tags, &[ItemKey::CopyrightMessage]),
        track_number: tags.iter().find_map(|tag| tag.track()),
        track_total: tags.iter().find_map(|tag| tag.track_total()),
        disc_number: tags.iter().find_map(|tag| tag.disk()),
        disc_total: tags.iter().find_map(|tag| tag.disk_total()),
        duration_ms: u64::try_from(props.duration().as_millis())
            .ok()
            .filter(|value| *value > 0),
        sample_rate: props.sample_rate(),
        channels: props.channels(),
        bit_depth: props.bit_depth(),
        bitrate_kbps: props.audio_bitrate().or(props.overall_bitrate()),
        ..Metadata::default()
    };
    if let Some(directory) = artwork {
        let picture = tags
            .iter()
            .flat_map(|tag| tag.pictures())
            .find(|picture| picture.pic_type() == PictureType::CoverFront)
            .or_else(|| tags.iter().flat_map(|tag| tag.pictures()).next());
        if let Some(picture) = picture {
            match cache_picture(directory, picture.data()) {
                Ok(key) => metadata.artwork = Some(key),
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "Cannot cache embedded artwork");
                    metadata.error = Some(format!("Artwork: {error:#}"));
                }
            }
        }
    }
    Ok((first(&tags, &[ItemKey::TrackTitle]), metadata))
}

fn cache_picture(directory: &Path, bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_PICTURE_BYTES {
        bail!("Embedded picture exceeds 16 MiB");
    }
    let key = format!("{:x}", Sha256::digest(bytes));
    let path = directory.join(format!("{key}.png"));
    let thumbnail = directory.join(format!("{key}-thumb.png"));
    if path.is_file() && thumbnail.is_file() {
        return Ok(key);
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let picture = reader.decode()?;
    fs::create_dir_all(directory)?;
    save_picture(directory, &path, &picture.thumbnail(1024, 1024))?;
    save_picture(directory, &thumbnail, &picture.thumbnail(128, 128))?;
    Ok(key)
}

fn save_picture(directory: &Path, path: &Path, picture: &image::DynamicImage) -> Result<()> {
    let mut png = Cursor::new(Vec::new());
    picture.write_to(&mut png, image::ImageFormat::Png)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(png.get_ref())?;
    file.as_file().sync_all()?;
    match file.persist_noclobber(path) {
        Ok(_) => {}
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.error.into()),
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
