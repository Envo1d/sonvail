use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, thread};

use rand::seq::SliceRandom;
use rodio::{Decoder, DeviceSinkBuilder, Player};

#[derive(Clone, Copy)]
enum LoopMode {
    Off,
    On,
}

fn get_music_dir() -> Option<PathBuf> {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let mut path = PathBuf::from(user_profile);
        path.push("Music");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn read_music_files() -> Vec<PathBuf> {
    let mut music_files = Vec::new();

    if let Some(music_dir) = get_music_dir()
        && let Ok(entries) = fs::read_dir(music_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
            {
                match ext.to_lowercase().as_str() {
                    "mp3" | "wav" | "flac" | "ogg" => music_files.push(path),
                    _ => {}
                }
            }
        }
    }

    music_files
}

fn main() -> anyhow::Result<()> {
    let sink = DeviceSinkBuilder::open_default_sink()?;

    let playlist: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let player = Arc::new(Mutex::new(Player::connect_new(sink.mixer())));
    let loop_mode = Arc::new(Mutex::new(LoopMode::Off));

    {
        let mut pl = playlist.lock().unwrap();
        let files = read_music_files();
        for file in files {
            pl.push(file.to_string_lossy().to_string());
        }
        println!("📂 Loaded {} tracks from Music folder", pl.len());
    }

    let playlist_clone = Arc::clone(&playlist);
    let player_clone = Arc::clone(&player);
    let loop_mode_clone = Arc::clone(&loop_mode);

    thread::spawn(move || {
        loop {
            if player_clone.lock().unwrap().empty() {
                let next_track = {
                    let mut pl = playlist_clone.lock().unwrap();
                    if pl.is_empty() {
                        drop(pl);
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    pl.remove(0)
                };

                println!("▶ Now playing: {next_track}");

                let file = match File::open(&next_track) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error opening {}: {}", next_track, e);
                        continue;
                    }
                };
                let source = match Decoder::new(BufReader::new(file)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error decoding {}: {}", next_track, e);
                        continue;
                    }
                };

                let s = player_clone.lock().unwrap();
                s.append(source);

                if let LoopMode::On = *loop_mode_clone.lock().unwrap() {
                    playlist_clone.lock().unwrap().push(next_track);
                }
            }

            thread::sleep(Duration::from_millis(200));
        }
    });

    println!("🎵 CLI MP3 Player");
    println!(
        "Commands: add <file> | play | pause | resume | skip | stop | loop on/off | shuffle | list | quit"
    );

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let cmd = line?.trim().to_string();
        let mut parts = cmd.split_whitespace();
        let action = parts.next();

        match action {
            Some("add") => {
                if let Some(path) = parts.next() {
                    playlist.lock().unwrap().push(path.to_string());
                    println!("Added: {}", path);
                } else {
                    println!("Usage: add <file>");
                }
            }
            Some("play") | Some("resume") => {
                player.lock().unwrap().play();
                println!("Playing");
            }
            Some("pause") => {
                player.lock().unwrap().pause();
                println!("Paused");
            }
            Some("skip") => {
                player.lock().unwrap().stop();
                println!("Skipped to next track");
            }
            Some("stop") => {
                player.lock().unwrap().stop();
                playlist.lock().unwrap().clear();
                println!("Stopped and cleared playlist");
            }
            Some("loop") => {
                if let Some(mode) = parts.next() {
                    let mut lm = loop_mode.lock().unwrap();
                    match mode {
                        "on" => {
                            *lm = LoopMode::On;
                            println!("Loop mode ON");
                        }
                        "off" => {
                            *lm = LoopMode::Off;
                            println!("Loop mode OFF");
                        }
                        _ => println!("Usage: loop on/off"),
                    }
                }
            }
            Some("shuffle") => {
                let mut pl = playlist.lock().unwrap();
                pl.shuffle(&mut rand::rng());
                println!("Shuffled playlist");
            }
            Some("list") => {
                let pl = playlist.lock().unwrap();
                if pl.is_empty() {
                    println!("Playlist is empty");
                } else {
                    println!("Playlist:");
                    for (i, track) in pl.iter().enumerate() {
                        println!("  {}. {}", i + 1, track);
                    }
                }
            }
            Some("quit") => {
                println!("Goodbye 👋");
                break;
            }
            _ => {
                println!(
                    "Unknown command. Commands: add <file> | play | pause | resume | skip | stop | loop on/off | shuffle | list | quit"
                );
            }
        }
    }

    Ok(())
}
