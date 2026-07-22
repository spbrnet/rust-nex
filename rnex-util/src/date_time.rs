use std::fmt::Display;

use bytemuck::{Pod, Zeroable};
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use tracing::error;

#[derive(Pod, Zeroable, Copy, Clone, Debug, Eq, PartialEq, Default)]
#[repr(transparent)]
pub struct DateTime(pub u64);

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{} {}:{}:{}",
            self.get_year(),
            self.get_month(),
            self.get_days(),
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds()
        )
    }
}

impl DateTime {
    // this is the time which smm returned as the expriy date, we use it as a
    // date so far into the future that it might as well just be never more generally
    pub const PRACTICALLY_NEVER: Self = Self::new(0, 0, 0, 31, 12, 9999);
    pub fn from_naive(dt: chrono::NaiveDateTime) -> Self {
        use chrono::Datelike;
        use chrono::Timelike;
        Self::new(
            dt.second() as u64,
            dt.minute() as u64,
            dt.hour() as u64,
            dt.day() as u64,
            dt.month() as u64,
            dt.year() as u64,
        )
    }

    pub const fn new(second: u64, minute: u64, hour: u64, day: u64, month: u64, year: u64) -> Self {
        Self(second | (minute << 6) | (hour << 12) | (day << 17) | (month << 22) | (year << 26))
    }

    pub fn now() -> Self {
        let now = chrono::Utc::now();
        Self::new(
            now.second() as u64,
            now.minute() as u64,
            now.hour() as u64,
            now.day() as u64,
            now.month() as u64,
            now.year() as u64,
        )
    }

    pub const fn get_seconds(&self) -> u8 {
        (self.0 & 0b11_1111) as u8
    }

    pub const fn get_minutes(&self) -> u8 {
        ((self.0 >> 6) & 0b11_1111) as u8
    }
    pub const fn get_hours(&self) -> u8 {
        ((self.0 >> 12) & 0b1_1111) as u8
    }
    pub const fn get_days(&self) -> u8 {
        ((self.0 >> 17) & 0b11_1111) as u8
    }
    pub const fn get_month(&self) -> u8 {
        ((self.0 >> 22) & 0b1111) as u8
    }
    pub const fn get_year(&self) -> u64 {
        (self.0 >> 26) & 0xFFFF_FFFF
    }
    pub fn to_regular_time(&self) -> chrono::DateTime<Utc> {
        let date = match NaiveDate::from_ymd_opt(
            self.get_year() as i32,
            self.get_month() as u32,
            self.get_days() as u32,
        ) {
            Some(v) => v,
            None => {
                error!("invalid datetime...: {}", self);
                Default::default()
            }
        };

        chrono::NaiveDateTime::new(
            date,
            chrono::NaiveTime::from_hms_opt(
                self.get_hours() as u32,
                self.get_minutes() as u32,
                self.get_seconds() as u32,
            )
            .unwrap_or_default(),
        )
        .and_utc()
    }
}
