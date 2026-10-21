use super::*;
use lofty::{
    prelude::TagExt,
    tag::{Tag, TagType},
};

/// Small silent PCM fixture; never sent to a real audio device.
pub(crate) fn wav(path: &Path) {
    let samples = 8_000u32;
    let size = samples * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&samples.to_le_bytes());
    bytes.extend_from_slice(&(samples * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.resize(44 + size as usize, 0);
    fs::write(path, bytes).unwrap();
}

#[test]
fn reads_audio_properties_and_tags_and_caches_unchanged_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("filename.wav");
    wav(&path);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.set_title("Song title".into());
    tag.set_artist("Artist".into());
    tag.set_album("Album".into());
    tag.insert_text(ItemKey::RecordingDate, "2025-07-12".into());
    tag.insert_text(ItemKey::Genre, "Electronic".into());
    tag.set_track(3);
    tag.set_disk(2);
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(3, 4)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    png.set_position(0);
    let mut picture = lofty::picture::Picture::from_reader(&mut png).unwrap();
    picture.set_pic_type(PictureType::CoverFront);
    tag.push_picture(picture);
    tag.save_to_path(&path, lofty::config::WriteOptions::default())
        .unwrap();
    let artwork = temp.path().join("artwork");
    let track = read_track(&path, None, Some(&artwork)).unwrap();
    assert!(track.metadata.artwork_path(&artwork).unwrap().is_file());
    assert_eq!(track.title, "Song title");
    assert_eq!(track.metadata.artists, ["Artist"]);
    assert_eq!(track.metadata.album.as_deref(), Some("Album"));
    assert_eq!(track.metadata.release_date.as_deref(), Some("2025-07-12"));
    assert_eq!(track.metadata.genres, ["Electronic"]);
    assert_eq!(track.metadata.track_number, Some(3));
    assert_eq!(track.metadata.disc_number, Some(2));
    assert_eq!(track.metadata.duration_ms, Some(1000));
    assert_eq!(track.metadata.sample_rate, Some(8000));
    assert_eq!(track.metadata.channels, Some(1));
    assert_eq!(track.metadata.bit_depth, Some(16));
    assert_eq!(
        read_track(&path, Some(&track), Some(&artwork)).unwrap(),
        track
    );
    tag.set_title("Updated title".into());
    tag.save_to_path(&path, lofty::config::WriteOptions::default())
        .unwrap();
    assert_eq!(
        read_track(&path, Some(&track), None).unwrap().title,
        "Updated title"
    );
}

#[test]
fn preserves_multiple_values_without_splitting_artist_names_on_slashes() {
    use lofty::tag::{ItemValue, TagItem};
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.push(TagItem::new(
        ItemKey::TrackArtist,
        ItemValue::Text(" AC/DC ".into()),
    ));
    tag.push(TagItem::new(
        ItemKey::TrackArtist,
        ItemValue::Text("Guest".into()),
    ));
    tag.push(TagItem::new(
        ItemKey::Genre,
        ItemValue::Text("Electronic".into()),
    ));
    tag.push(TagItem::new(
        ItemKey::Genre,
        ItemValue::Text("Ambient".into()),
    ));
    assert_eq!(values(&[&tag], &[ItemKey::TrackArtist]), ["AC/DC", "Guest"]);
    assert_eq!(
        values(&[&tag], &[ItemKey::Genre]),
        ["Electronic", "Ambient"]
    );
}

#[test]
fn malformed_files_keep_filename_fallback_without_stopping_indexing() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("broken.mp3");
    fs::write(&path, b"not audio").unwrap();
    let track = read_track(&path, None, None).unwrap();
    assert_eq!(track.title, "broken");
    assert!(track.metadata.error.is_some());
}

#[test]
fn caches_bounded_artwork_by_content_and_rejects_invalid_images() {
    let temp = tempfile::tempdir().unwrap();
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 3)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let key = cache_picture(temp.path(), bytes.get_ref()).unwrap();
    assert_eq!(cache_picture(temp.path(), bytes.get_ref()).unwrap(), key);
    assert!(temp.path().join(format!("{key}.png")).is_file());
    let thumbnail = image::ImageReader::open(temp.path().join(format!("{key}-thumb.png")))
        .unwrap()
        .decode()
        .unwrap();
    assert!(thumbnail.width() <= 128 && thumbnail.height() <= 128);
    assert!(cache_picture(temp.path(), b"invalid").is_err());
    assert!(
        Metadata {
            artwork: Some("../outside".into()),
            ..Metadata::default()
        }
        .artwork_path(temp.path())
        .is_none()
    );
}
