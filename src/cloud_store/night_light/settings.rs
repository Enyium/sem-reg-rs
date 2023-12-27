use std::time::SystemTime;
use winreg::enums::HKEY_CURRENT_USER;

use super::{
    time::{BinConvertClockTime, ClockTime, ClockTimeFrame},
    NightLight,
};
use crate::{
    cloud_store::prologue::CloudStoreValuePrologue,
    data_conversion::{
        byte_seq::{ByteSeq, ParseError},
        time::{
            epoch_duration_to_epoch_secs, now_as_epoch_duration, system_time_to_epoch_duration,
        },
        ResultOrElseIf, Strictness, TrackedValue,
    },
    reg::{read_reg_bin_value, RegValuePath},
};

#[derive(PartialEq, Debug)]
pub struct RawNightLightSettings {
    pub prologue_epoch_secs: u32,
    pub schedule_active: TrackedValue<bool>,
    pub schedule_type: TrackedValue<ScheduleType>,
    pub scheduled_night: TrackedValue<ClockTimeFrame>,
    pub night_color_temp: TrackedValue<Option<u16>>,
    pub sunset_to_sunrise: Option<ClockTimeFrame>,
    pub night_preview_active: TrackedValue<bool>,
}

impl RawNightLightSettings {
    pub const REG_VALUE_PATH: RegValuePath<'_> = RegValuePath {
        hkey: HKEY_CURRENT_USER,
        subkey_path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.settings\windows.data.bluelightreduction.settings",
        value_name: "Data",
    };

    pub fn from_reg(strictness: Strictness) -> Result<Self, super::Error> {
        Ok(Self::from_bytes(
            read_reg_bin_value(&Self::REG_VALUE_PATH)?,
            strictness,
        )?)
    }

    pub fn from_bytes(bytes: Vec<u8>, strictness: Strictness) -> Result<Self, ParseError> {
        let mut byte_seq = ByteSeq::from_bytes(bytes);

        let prologue = CloudStoreValuePrologue::from_byte_seq(&mut byte_seq, strictness)?;
        let prologue_epoch_secs = prologue.epoch_secs.ok_or(ParseError::InconsistentData)?;
        prologue
            .num_body_bytes
            .ok_or(ParseError::InconsistentData)
            .or_else_if(strictness.is_lenient(), |_| Ok(0))?;

        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        let schedule_active = TrackedValue::new(byte_seq.assert_const(&[0x02, 0x01]).is_ok());
        let schedule_type =
            TrackedValue::new(if byte_seq.assert_const(&[0xc2, 0x0a, 0x00]).is_ok() {
                ScheduleType::Explicit
            } else {
                ScheduleType::SunsetToSunrise
            });

        let const_error_to_midnight = |error| match error {
            ParseError::ExpectedConst(_) => Ok(ClockTime::MIDNIGHT),
            _ => Err(error),
        };
        let scheduled_night = TrackedValue::new(ClockTimeFrame {
            start: {
                byte_seq
                    .assert_const(&[0xca, 0x14])
                    .and_then(|_| byte_seq.read_clock_time())
                    .or_else_if(strictness.is_lenient(), const_error_to_midnight)?
            },
            end: {
                byte_seq
                    .assert_zero()
                    .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
                byte_seq
                    .assert_const(&[0xca, 0x1e])
                    .and_then(|_| byte_seq.read_clock_time())
                    .or_else_if(strictness.is_lenient(), const_error_to_midnight)?
            },
        });

        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        let night_color_temp = TrackedValue::new(if byte_seq.assert_const(&[0xcf, 0x28]).is_ok() {
            Some(
                byte_seq
                    .read_zigzag_vlq_64()?
                    .try_into()
                    .map_err(|_| ParseError::ValueNotInRange)?,
            )
        } else {
            None
        });

        let sunset_time = byte_seq
            .assert_const(&[0xca, 0x32])
            .and_then(|_| byte_seq.read_clock_time())
            .or_else_if(strictness.is_lenient(), const_error_to_midnight)?;
        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        let sunrise_time = byte_seq
            .assert_const(&[0xca, 0x3c])
            .and_then(|_| byte_seq.read_clock_time())
            .or_else_if(strictness.is_lenient(), const_error_to_midnight)?;
        let sunset_to_sunrise = if sunset_time.is_midnight() && sunrise_time.is_midnight() {
            None
        } else {
            Some(ClockTimeFrame {
                start: sunset_time,
                end: sunrise_time,
            })
        };

        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        let night_preview_active =
            TrackedValue::new(byte_seq.assert_const(&[0xc2, 0x46]).map_or_else(
                |_| Ok(false),
                |_| {
                    byte_seq.assert_const(&[0x01]).map_or_else(
                        |error| {
                            if strictness.is_lenient() {
                                Ok(false)
                            } else {
                                Err(error)
                            }
                        },
                        |_| Ok(true),
                    )
                },
            )?);

        (0..4)
            .try_for_each(|_| byte_seq.assert_zero())
            .and_then(|_| byte_seq.assert_exhausted())
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;

        Ok(Self {
            prologue_epoch_secs,
            schedule_active,
            schedule_type,
            scheduled_night,
            night_color_temp,
            sunset_to_sunrise,
            night_preview_active,
        })
    }

    pub fn lenient_fallback(now: SystemTime) -> Self {
        Self {
            prologue_epoch_secs: epoch_duration_to_epoch_secs(system_time_to_epoch_duration(now)),
            schedule_active: TrackedValue::new(false),
            schedule_type: TrackedValue::new(ScheduleType::SunsetToSunrise),
            scheduled_night: TrackedValue::new(ClockTimeFrame {
                // Default according to <https://thegeekpage.com/how-to-set-a-schedule-to-turn-on-night-light-in-windows-11/>.
                start: ClockTime::from_h_min(21, 0).unwrap(),
                end: ClockTime::from_h_min(7, 0).unwrap(),
            }),
            night_color_temp: TrackedValue::new(Some(NightLight::DEFAULT_NIGHT_COLOR_TEMP)),
            sunset_to_sunrise: None,
            night_preview_active: TrackedValue::new(false),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        const MAX_BODY_LEN: usize = 45;
        let mut body_byte_seq = ByteSeq::with_capacity(MAX_BODY_LEN);

        body_byte_seq.push_zero();
        if *self.schedule_active {
            body_byte_seq.push_const(&[0x02, 0x01]);
        }
        if *self.schedule_type == ScheduleType::Explicit {
            body_byte_seq.push_const(&[0xc2, 0x0a, 0x00]);
        }

        body_byte_seq.push_const(&[0xca, 0x14]);
        body_byte_seq.push_clock_time(self.scheduled_night.start);

        body_byte_seq.push_zero();
        body_byte_seq.push_const(&[0xca, 0x1e]);
        body_byte_seq.push_clock_time(self.scheduled_night.end);

        body_byte_seq.push_zero();
        if let Some(night_color_temp) = *self.night_color_temp {
            body_byte_seq.push_const(&[0xcf, 0x28]);
            body_byte_seq.push_zigzag_vlq_64(night_color_temp as _);
        }

        let sunset_to_sunrise = self
            .sunset_to_sunrise
            .unwrap_or_else(|| ClockTimeFrame::MIDNIGHT_TO_MIDNIGHT);
        body_byte_seq.push_const(&[0xca, 0x32]);
        body_byte_seq.push_clock_time(sunset_to_sunrise.start);

        body_byte_seq.push_zero();
        body_byte_seq.push_const(&[0xca, 0x3c]);
        body_byte_seq.push_clock_time(sunset_to_sunrise.end);

        body_byte_seq.push_zero();
        if *self.night_preview_active {
            body_byte_seq.push_const(&[0xc2, 0x46, 0x01]);
        }

        for _ in 0..4 {
            body_byte_seq.push_zero();
        }

        let mut byte_seq = CloudStoreValuePrologue {
            epoch_secs: Some(
                epoch_duration_to_epoch_secs(now_as_epoch_duration())
                    .max(self.prologue_epoch_secs + 2),
            ),
            num_body_bytes: Some(body_byte_seq.len() as _),
        }
        .to_byte_seq(Some(MAX_BODY_LEN));
        byte_seq.extend(&body_byte_seq);

        byte_seq.into()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ScheduleType {
    /// Based on the user's location.
    SunsetToSunrise,
    /// Explicitly chosen clock times. Also used as the fallback when the other variant is unavailable because location services are turned off.
    Explicit,
}

#[cfg(test)]
mod tests {
    use super::RawNightLightSettings;
    use crate::{
        cloud_store::night_light::{
            settings::ScheduleType,
            time::{ClockTime, ClockTimeFrame},
        },
        data_conversion::Strictness,
    };

    #[test]
    fn shortest_from_and_to_bytes() {
        let bytes = [
            0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xfe, 0xcf, 0xee, 0xa9,
            0x06, 0x2a, 0x2b, 0x0e, 0x11, 0x43, 0x42, 0x01, 0x00, 0xca, 0x14, 0x00, 0xca, 0x1e,
            0x00, 0xca, 0x32, 0x00, 0xca, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            match RawNightLightSettings::from_bytes(bytes.to_vec(), strictness) {
                Ok(settings) => {
                    assert_eq!(*settings.schedule_active, false);
                    assert_eq!(*settings.schedule_type, ScheduleType::SunsetToSunrise);
                    assert_eq!(
                        *settings.scheduled_night,
                        ClockTimeFrame::MIDNIGHT_TO_MIDNIGHT
                    );
                    assert_eq!(*settings.night_color_temp, None);
                    assert_eq!(settings.sunset_to_sunrise, None);
                    assert_eq!(*settings.night_preview_active, false);

                    assert_eq!(settings.to_bytes().len(), bytes.len());
                }
                result => panic!("{result:?}"),
            }
        }
    }

    #[test]
    fn longest_from_and_to_bytes() {
        let bytes = [
            0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xfe, 0xcf, 0xee, 0xa9,
            0x06, 0x2a, 0x2b, 0x0e, 0x2d, 0x43, 0x42, 0x01, 0x00, 0x02, 0x01, 0xc2, 0x0a, 0x00,
            0xca, 0x14, 0x0e, 0x08, 0x2e, 0x0f, 0x00, 0xca, 0x1e, 0x0e, 0x0e, 0x2e, 0x1e, 0x00,
            0xcf, 0x28, 0xf8, 0x29, 0xca, 0x32, 0x0e, 0x15, 0x2e, 0x03, 0x00, 0xca, 0x3c, 0x0e,
            0x06, 0x2e, 0x14, 0x00, 0xc2, 0x46, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            match RawNightLightSettings::from_bytes(bytes.to_vec(), strictness) {
                Ok(settings) => {
                    assert_eq!(*settings.schedule_active, true);
                    assert_eq!(*settings.schedule_type, ScheduleType::Explicit);
                    assert_eq!(
                        *settings.scheduled_night,
                        ClockTimeFrame {
                            start: ClockTime::from_h_min(8, 15).unwrap(),
                            end: ClockTime::from_h_min(14, 30).unwrap(),
                        }
                    );
                    assert_eq!(*settings.night_color_temp, Some(2684));
                    assert_eq!(
                        settings.sunset_to_sunrise,
                        Some(ClockTimeFrame {
                            start: ClockTime::from_h_min(21, 3).unwrap(),
                            end: ClockTime::from_h_min(6, 20).unwrap(),
                        })
                    );
                    assert_eq!(*settings.night_preview_active, true);

                    assert_eq!(settings.to_bytes().len(), bytes.len());
                }
                result => panic!("{result:?}"),
            }
        }
    }
}
