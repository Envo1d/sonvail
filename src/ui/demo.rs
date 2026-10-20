// Presentation-only fixtures. These are not loaded tracks or a live playback state.
pub struct Track {
    pub title: &'static str,
    pub duration: &'static str,
    pub color: u32,
}

pub const ARTIST: &str = "ODESZA";
pub const ALBUM: &str = "A Moment Apart";
pub const CURRENT_TRACK: usize = 1;

pub const UP_NEXT: &[Track] = &[
    Track {
        title: "Line Of Sight (feat. WYNNE & Mansionair)",
        duration: "3:57",
        color: 0x665c80,
    },
    Track {
        title: "Late Night",
        duration: "3:48",
        color: 0x3c426a,
    },
    Track {
        title: "Across The Room (feat. Leon Bridges)",
        duration: "4:43",
        color: 0x61505f,
    },
];

pub const TRACKS: &[Track] = &[
    Track {
        title: "Intro",
        duration: "1:31",
        color: 0x6e466f,
    },
    Track {
        title: ALBUM,
        duration: "3:53",
        color: 0x785986,
    },
    Track {
        title: "Higher Ground (feat. Naomi Wild)",
        duration: "3:35",
        color: 0x927b85,
    },
    Track {
        title: "Boy",
        duration: "3:05",
        color: 0x355572,
    },
    Track {
        title: "Line Of Sight (feat. WYNNE & Mansionair)",
        duration: "3:57",
        color: 0x665c80,
    },
    Track {
        title: "Late Night",
        duration: "3:48",
        color: 0x3c426a,
    },
];

pub const QUEUE: &[Track] = &[
    Track {
        title: "Sun Models (feat. Madelyn Grant)",
        duration: "2:41",
        color: 0x654566,
    },
    Track {
        title: "For Us (feat. Briana Marela)",
        duration: "3:28",
        color: 0x827677,
    },
    Track {
        title: "All We Need (feat. Shy Girls)",
        duration: "3:31",
        color: 0x325674,
    },
    Track {
        title: "Divide (feat. Kelsey Bulkin)",
        duration: "3:46",
        color: 0x7f463f,
    },
    Track {
        title: "Meridian",
        duration: "3:00",
        color: 0x795e3c,
    },
];
