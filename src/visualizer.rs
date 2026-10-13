use std::fs;
use std::thread;
use std::time::Duration;

use amane::Service;

pub const BARS: usize = 24;

// cava prints one line per frame: BARS numbers from 0 to 100, separated by ';'
const CONFIG: &str = "\
[general]
bars = 24
framerate = 30
sleep_timer = 2

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 100
bar_delimiter = 59
frame_delimiter = 10

[smoothing]
noise_reduction = 70
";

pub struct Visualizer {
    pub levels: [u8; BARS],
}

impl Service for Visualizer {
    fn new() -> Self {
        Self { levels: [0; BARS] }
    }

    // cava does the listening; only frames that differ wake the windows reading this
    fn listen() {
        let path = format!("{}/amane-cava.conf", std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| String::from("/tmp")));
        if fs::write(&path, CONFIG).is_err() {
            return;
        }

        loop {
            for line in amane::lines(&format!("cava -p '{path}'")) {
                let mut levels = [0u8; BARS];
                for (slot, value) in levels.iter_mut().zip(line.split(';')) {
                    *slot = value.trim().parse().unwrap_or(0);
                }
                if Self::read().levels != levels {
                    Self::write().levels = levels;
                }
            }
            // cava quit or crashed, try again shortly
            thread::sleep(Duration::from_secs(5));
        }
    }
}
