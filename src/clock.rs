use std::thread;
use std::time::Duration;

use amane::Service;

// one known new moon (2000-01-06 18:14 UTC) and the length of a lunar cycle
const NEW_MOON_EPOCH: f64 = 947_182_440.0;
const SYNODIC_DAYS: f64 = 29.530_588_853;

#[derive(Clone, PartialEq, Default)]
pub struct Clock {
    pub epoch: i64,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    // 1 is Monday, 7 is Sunday
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
    pub first_weekday: u32,
    pub days_in_month: u32,
}

impl Clock {
    fn sample() -> Self {
        let now = amane::output("date '+%s %Y %m %d %u %H %M'");
        let fields: Vec<&str> = now.split_whitespace().collect();
        if fields.len() < 7 {
            return Self::default();
        }

        let number = |index: usize| fields[index].trim_start_matches('0').parse::<u32>().unwrap_or(0);
        let (year, month) = (fields[1], fields[2]);

        let first_weekday = amane::output(&format!("date -d '{year}-{month}-01' +%u"))
            .trim()
            .parse()
            .unwrap_or(1);
        let days_in_month = amane::output(&format!("date -d '{year}-{month}-01 +1 month -1 day' +%d"))
            .trim()
            .parse()
            .unwrap_or(30);

        Self {
            epoch: fields[0].parse().unwrap_or(0),
            year: year.parse().unwrap_or(2000),
            month: number(2),
            day: number(3),
            weekday: number(4),
            hour: number(5),
            minute: number(6),
            first_weekday,
            days_in_month,
        }
    }

    pub fn weekday_name(&self) -> &'static str {
        short_weekday(self.weekday)
    }

    pub fn dark_hour(&self) -> bool {
        self.hour == 0
    }

    pub fn period(&self) -> &'static str {
        match self.hour {
            0 => "DARK HOUR",
            1..=4 => "LATE NIGHT",
            5..=6 => "EARLY MORNING",
            7..=11 => "MORNING",
            12 => "LUNCHTIME",
            13..=14 => "AFTERNOON",
            15..=17 => "AFTER SCHOOL",
            _ => "EVENING",
        }
    }

    // 0 is a new moon, 0.5 is full
    pub fn moon_phase(&self) -> f32 {
        let days = (self.epoch as f64 - NEW_MOON_EPOCH) / 86_400.0;
        (days / SYNODIC_DAYS).rem_euclid(1.0) as f32
    }

    pub fn moon_name(&self) -> &'static str {
        const NAMES: [&str; 8] = [
            "NEW MOON",
            "WAXING CRESCENT",
            "FIRST QUARTER",
            "WAXING GIBBOUS",
            "FULL MOON",
            "WANING GIBBOUS",
            "LAST QUARTER",
            "WANING CRESCENT",
        ];
        NAMES[((self.moon_phase() * 8.0 + 0.5) as usize) % 8]
    }

    pub fn days_to_full(&self) -> u32 {
        let phase = self.moon_phase() as f64;
        let remaining = if phase <= 0.5 { 0.5 - phase } else { 1.5 - phase };
        (remaining * SYNODIC_DAYS).round() as u32
    }
}

pub fn short_weekday(weekday: u32) -> &'static str {
    match weekday {
        1 => "MON",
        2 => "TUE",
        3 => "WED",
        4 => "THU",
        5 => "FRI",
        6 => "SAT",
        _ => "SUN",
    }
}

impl Service for Clock {
    fn new() -> Self {
        Self::sample()
    }

    // samples outside the lock so a slow `date` never blocks a frame; redraws once a minute
    fn listen() {
        loop {
            thread::sleep(Duration::from_secs(1));
            let next = Self::sample();
            let changed = {
                let current = Self::read();
                current.minute != next.minute || current.day != next.day
            };
            if changed {
                *Self::write() = next;
            }
        }
    }
}
