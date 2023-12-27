use std::{num::ParseIntError, str::FromStr};

use map_self::MapSelf;
use serde::Serialize;
use thiserror::Error;

use crate::data_conversion::byte_seq::{ByteSeq, ParseError};

#[derive(Clone, Copy, PartialEq, Serialize, Debug)]
pub struct ClockTimeFrame {
    pub start: ClockTime,
    pub end: ClockTime,
}

impl ClockTimeFrame {
    pub const MIDNIGHT_TO_MIDNIGHT: Self = Self {
        start: ClockTime::MIDNIGHT,
        end: ClockTime::MIDNIGHT,
    };

    pub fn format(&self, use_12_hour_clock: bool) -> String {
        format!(
            "{}-{}",
            self.start.format(use_12_hour_clock),
            self.end.format(use_12_hour_clock)
        )
    }
}

impl FromStr for ClockTimeFrame {
    type Err = ClockTimeOrFrameFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut clock_time_iter = s.splitn(2, '-');
        if let (Some(start), Some(end)) = (clock_time_iter.next(), clock_time_iter.next()) {
            Ok(Self {
                start: start.parse()?,
                end: end.parse()?,
            })
        } else {
            Err(ClockTimeOrFrameFromStrError)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Debug)]
pub struct ClockTime {
    pub(super) hour: u8,
    pub(super) minute: u8,
}

impl ClockTime {
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0 };

    pub fn from_h_min(hour: u8, minute: u8) -> Option<Self> {
        if hour <= 23 && minute <= 59 {
            Some(Self { hour, minute })
        } else {
            None
        }
    }

    pub fn from_h_min_with_meridiem(mut hour: u8, minute: u8, meridiem: Meridiem) -> Option<Self> {
        if hour > 12 {
            return None;
        }

        match (meridiem, &mut hour) {
            (Meridiem::Am, hour) if *hour == 12 => *hour = 0,
            (Meridiem::Pm, hour) if *hour < 12 => *hour += 12,
            _ => {}
        };

        Self::from_h_min(hour, minute)
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn hour_meridiem(&self) -> (u8, Meridiem) {
        if self.hour == 0 {
            (12, Meridiem::Am)
        } else if self.hour == 12 {
            (12, Meridiem::Pm)
        } else if self.hour > 12 {
            (self.hour - 12, Meridiem::Pm)
        } else {
            (self.hour, Meridiem::Am)
        }
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    pub fn is_midnight(&self) -> bool {
        self.hour == 0 && self.minute == 0
    }

    pub fn format(&self, use_12_hour_clock: bool) -> String {
        let (hour, meridiem) = if use_12_hour_clock {
            self.hour_meridiem()
                .map_self(|(hour, meridiem)| (hour, Some(meridiem)))
        } else {
            (self.hour, None)
        };

        let mut string = format!("{:02}:{:02}", hour, self.minute,);

        if let Some(meridiem) = meridiem {
            string.push_str(meridiem.as_str());
        }

        string
    }
}

impl FromStr for ClockTime {
    type Err = ClockTimeOrFrameFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase_string = s.to_ascii_lowercase();
        let mut number_iter = lowercase_string.splitn(2, ':');

        if let (Some(hour), Some(minute_meridiem)) = (number_iter.next(), number_iter.next()) {
            let hour = hour.parse()?;

            let (minute, meridiem) = if let Some(minute) = minute_meridiem.strip_suffix("am") {
                (minute, Some(Meridiem::Am))
            } else if let Some(minute) = minute_meridiem.strip_suffix("pm") {
                (minute, Some(Meridiem::Pm))
            } else {
                (minute_meridiem, None)
            };
            let minute = minute.parse()?;

            Ok(if let Some(meridiem) = meridiem {
                Self::from_h_min_with_meridiem(hour, minute, meridiem)
            } else {
                Self::from_h_min(hour, minute)
            }
            .ok_or(ClockTimeOrFrameFromStrError)?)
        } else {
            Err(ClockTimeOrFrameFromStrError)
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Meridiem {
    Am,
    Pm,
}

impl Meridiem {
    fn as_str(&self) -> &'static str {
        match self {
            Meridiem::Am => "am",
            Meridiem::Pm => "pm",
        }
    }
}

pub(super) trait BinConvertClockTime {
    fn read_clock_time(&mut self) -> Result<ClockTime, ParseError>;
    fn push_clock_time(&mut self, clock_time: ClockTime);
}

impl BinConvertClockTime for ByteSeq {
    fn read_clock_time(&mut self) -> Result<ClockTime, ParseError> {
        let hour = if self.assert_const(&[0x0e]).is_ok() {
            self.read_int()?
        } else {
            0
        };

        let minute = if self.assert_const(&[0x2e]).is_ok() {
            self.read_int()?
        } else {
            0
        };

        Ok(ClockTime::from_h_min(hour, minute).ok_or(ParseError::ValueNotInRange)?)
    }

    fn push_clock_time(&mut self, clock_time: ClockTime) {
        if clock_time.hour != 0 {
            self.push_const(&[0x0e]);
            self.push_int(clock_time.hour);
        }
        if clock_time.minute != 0 {
            self.push_const(&[0x2e]);
            self.push_int(clock_time.minute);
        }
    }
}

#[derive(Error, PartialEq, Debug)]
#[error("couldn't parse `ClockTime` or `ClockTimeFrame`")]
pub struct ClockTimeOrFrameFromStrError;

impl From<ParseIntError> for ClockTimeOrFrameFromStrError {
    fn from(_error: ParseIntError) -> Self {
        ClockTimeOrFrameFromStrError
    }
}

#[cfg(test)]
mod tests {
    use crate::cloud_store::night_light::{ClockTime, ClockTimeFrame, Meridiem};

    #[test]
    fn clock_time_frame_from_str() {
        assert_eq!(
            "20:21-6:00".parse::<ClockTimeFrame>(),
            Ok(ClockTimeFrame {
                start: ClockTime::from_h_min_with_meridiem(8, 21, Meridiem::Pm).unwrap(),
                end: ClockTime::from_h_min_with_meridiem(6, 0, Meridiem::Am).unwrap()
            })
        );
        assert_eq!(
            "08:00pm-05:45AM".parse::<ClockTimeFrame>(),
            Ok(ClockTimeFrame {
                start: ClockTime::from_h_min(20, 00).unwrap(),
                end: ClockTime::from_h_min(5, 45).unwrap()
            })
        );
        assert_eq!(
            "9:59-9:59am".parse::<ClockTimeFrame>(),
            Ok(ClockTimeFrame {
                start: ClockTime::from_h_min_with_meridiem(9, 59, Meridiem::Am).unwrap(),
                end: ClockTime::from_h_min(9, 59).unwrap()
            })
        );

        assert!("9:59-9:59am-".parse::<ClockTimeFrame>().is_err());
        assert!("10:00 - 10:00".parse::<ClockTimeFrame>().is_err());
        assert!("10.00-10.00".parse::<ClockTimeFrame>().is_err());
        assert!("10:00-10:60".parse::<ClockTimeFrame>().is_err());
        assert!("10:00-24:00".parse::<ClockTimeFrame>().is_err());
    }
}
