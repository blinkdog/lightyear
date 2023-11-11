use bitcode::{Decode, Encode};
use chrono::Duration as ChronoDuration;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::time::Duration;

// TODO: still keep around the old wrapped time with wrapping

// wrapping time: u32::MAX in microseconds (a bit over an hour)
pub const WRAPPING_TIME_US: u32 = u32::MAX;

pub struct TimeManager {
    wrapped_time: WrappedTime,
}

impl TimeManager {
    pub fn new() -> Self {
        Self {
            wrapped_time: WrappedTime::new(0),
        }
    }

    /// Update the time by matching the virtual time from bevy
    /// (time from server start, wrapped around the hour)
    pub fn update(&mut self, delta: Duration) {
        self.wrapped_time += delta;
    }

    // pub fn subtract_millis(&mut self, offset_ms: u32) {
    //     let add_millis = WRAPPING_TIME_MS - offset_ms;
    //     self.wrapped_time.elapsed_ms_wrapped += add_millis;
    // }

    /// Current time since server start, wrapped around 1 hour
    pub fn current_time(&self) -> WrappedTime {
        self.wrapped_time
    }

    pub fn set_current_time(&mut self, time: WrappedTime) {
        self.wrapped_time = time;
    }

    // /// Current time since server start, wrapped around 1 hour
    // pub fn mut_current_time(&mut self) -> WrappedTime {
    //     mut self.wrapped_time
    // }
}

/// Time since start of server, in milliseconds
/// Serializes in a compact manner
#[derive(Encode, Decode, Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq)]
pub struct WrappedTime {
    // Amount of time elapsed since the start of the server, in microseconds
    // wraps around 1 hour
    // We use milli-seconds because micro-seconds lose precisions very quickly
    // #[bitcode_hint(expected_range = "0..3600000000")]
    elapsed_us_wrapped: u32,
}

impl WrappedTime {
    pub fn new(elapsed_us_wrapped: u32) -> Self {
        Self { elapsed_us_wrapped }
    }

    pub fn from_duration(elapsed_wrapped: Duration) -> Self {
        // TODO: check cast?
        let elapsed_us_wrapped = elapsed_wrapped.as_micros() as u32;
        Self { elapsed_us_wrapped }
    }

    pub fn to_duration(&self) -> Duration {
        Duration::from_micros(self.elapsed_us_wrapped as u64)
    }

    /// Returns time b - time a, in microseconds
    /// Can be positive if b is in the future, or negative is b is in the past
    pub fn wrapping_diff(a: &Self, b: &Self) -> i32 {
        const MAX: i32 = (WRAPPING_TIME_US / 2 - 1) as i32;
        const MIN: i32 = -MAX;
        const ADJUST: i32 = WRAPPING_TIME_US as i32;

        let a: i32 = a.elapsed_us_wrapped as i32;
        let b: i32 = b.elapsed_us_wrapped as i32;

        let mut result = b - a;
        if (MIN..=MAX).contains(&result) {
            result
        } else if b > a {
            result = b - (a + ADJUST);
            if (MIN..=MAX).contains(&result) {
                result
            } else {
                panic!("integer overflow, this shouldn't happen")
            }
        } else {
            result = (b + ADJUST) - a;
            if (MIN..=MAX).contains(&result) {
                result
            } else {
                panic!("integer overflow, this shouldn't happen")
            }
        }
    }
}

impl Ord for WrappedTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match Self::wrapping_diff(self, other) {
            0 => Ordering::Equal,
            x if x > 0 => Ordering::Less,
            x if x < 0 => Ordering::Greater,
            _ => unreachable!(),
        }
    }
}

impl PartialOrd for WrappedTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Returns the absolute duration between two times (no matter which one is ahead of which)!
impl Sub for WrappedTime {
    type Output = ChronoDuration;

    fn sub(self, rhs: Self) -> Self::Output {
        let diff_us = Self::wrapping_diff(&rhs, &self);
        ChronoDuration::microseconds(diff_us as i64)
    }
}

/// Returns the absolute duration between two times (no matter which one is ahead of which)!
/// Only valid for durations under 1 hour
impl SubAssign<ChronoDuration> for WrappedTime {
    fn sub_assign(&mut self, rhs: ChronoDuration) {
        let rhs_micros = rhs.num_microseconds().unwrap();
        // we can use wrapping_sub because we wrap around u32::max
        if rhs_micros > 0 {
            self.elapsed_us_wrapped = self.elapsed_us_wrapped.wrapping_sub(rhs_micros as u32);
        } else {
            self.elapsed_us_wrapped = self.elapsed_us_wrapped.wrapping_add(rhs_micros as u32);
        }
    }
}

impl Add<Duration> for WrappedTime {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            elapsed_us_wrapped: self.elapsed_us_wrapped.wrapping_add(rhs.as_micros() as u32),
        }
    }
}

impl Add<ChronoDuration> for WrappedTime {
    type Output = Self;

    fn add(self, rhs: ChronoDuration) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl AddAssign<ChronoDuration> for WrappedTime {
    fn add_assign(&mut self, rhs: ChronoDuration) {
        let rhs_micros = rhs.num_microseconds().unwrap();
        if rhs_micros > 0 {
            self.elapsed_us_wrapped = self.elapsed_us_wrapped.wrapping_add(rhs_micros as u32);
        } else {
            self.elapsed_us_wrapped = self.elapsed_us_wrapped.wrapping_sub(rhs_micros as u32);
        }
    }
}

impl AddAssign<Duration> for WrappedTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.elapsed_us_wrapped = self.elapsed_us_wrapped.wrapping_add(rhs.as_micros() as u32);
    }
}

impl From<Duration> for WrappedTime {
    fn from(value: Duration) -> Self {
        Self::from_duration(value)
    }
}

impl From<WrappedTime> for Duration {
    fn from(value: WrappedTime) -> Self {
        value.to_duration()
    }
}
