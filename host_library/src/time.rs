extern crate ffmpeg;

use ffmpeg::util::mathematics::rescale::{Rescale, TIME_BASE};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Time {
    time_base: ffmpeg::Rational,
    time: Option<i64>,
}

impl Time {
    pub fn from_secs_f64(secs: f64) -> Self {
        Self {
            time: Some((secs * TIME_BASE.denominator() as f64).round() as i64),
            time_base: TIME_BASE,
        }
    }

    /// Create a new zero-valued timestamp.
    pub fn zero() -> Self {
        Time {
            time: Some(0),
            time_base: (1, 90000).into(),
        }
    }

    pub fn aligned_with(&self, rhs: &Time) -> Aligned {
        Aligned {
            lhs: self.time,
            rhs: rhs
                .time
                .map(|rhs_time| rhs_time.rescale(rhs.time_base, self.time_base)),
            time_base: self.time_base,
        }
    }

    pub fn into_value(self) -> Option<i64> {
        self.time
    }

    pub(crate) fn aligned_with_rational(&self, time_base: ffmpeg::Rational) -> Time {
        Time {
            time: self
                .time
                .map(|time| time.rescale(self.time_base, time_base)),
            time_base,
        }
    }
}

impl From<Duration> for Time {
    /// Convert from a [`Duration`] to [`Time`].
    #[inline]
    fn from(duration: Duration) -> Self {
        Time::from_secs_f64(duration.as_secs_f64())
    }
}

//

impl std::fmt::Display for Time {
    /// Format [`Time`] as follows:
    ///
    /// * If the inner value is not `None`: `time/time_base`.
    /// * If the inner value is `None`: `none`.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(time) = self.time {
            let num = self.time_base.numerator() as i64 * time;
            let den = self.time_base.denominator();
            write!(f, "{num}/{den} secs")
        } else {
            write!(f, "none")
        }
    }
}

/// This is a virtual object that represents two aligned times.
///
/// On this object, arthmetic operations can be performed that operate on the two contained times.
/// This virtual object ensures that the interface to these operations is safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aligned {
    lhs: Option<i64>,
    rhs: Option<i64>,
    time_base: ffmpeg::Rational,
}

impl Aligned {
    /// Add two timestamps together.
    pub fn add(self) -> Time {
        self.apply(|lhs, rhs| lhs + rhs)
    }

    fn apply<F>(self, f: F) -> Time
    where
        F: FnOnce(i64, i64) -> i64,
    {
        match (self.lhs, self.rhs) {
            (Some(lhs_time), Some(rhs_time)) => Time {
                time: Some(f(lhs_time, rhs_time)),
                time_base: self.time_base,
            },
            _ => Time {
                time: None,
                time_base: self.time_base,
            },
        }
    }
}
