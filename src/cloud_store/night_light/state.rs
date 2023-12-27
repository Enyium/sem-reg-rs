use std::time::SystemTime;

use winreg::enums::HKEY_CURRENT_USER;

use crate::{
    cloud_store::prologue::CloudStoreValuePrologue,
    data_conversion::{
        byte_seq::{ByteSeq, ParseError},
        time::{
            epoch_duration_to_epoch_secs, epoch_duration_to_filetime, now_as_epoch_duration,
            system_time_to_epoch_duration, LATEST_FILETIME,
        },
        ResultOrElseIf, Strictness, TrackedValue,
    },
    reg::{read_reg_bin_value, RegValuePath},
};

#[derive(PartialEq, Debug)]
pub struct RawNightLightState {
    pub prologue_epoch_secs: u32,
    pub active: TrackedValue<bool>,
    pub transition_cause: TransitionCause,
    pub modified_filetime: i64,
}

impl RawNightLightState {
    pub const REG_VALUE_PATH: RegValuePath<'_> = RegValuePath {
        hkey: HKEY_CURRENT_USER,
        subkey_path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate",
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
        let active = TrackedValue::new(byte_seq.assert_const(&[0x10, 0x00]).is_ok());
        let transition_cause = if byte_seq.assert_const(&[0xd0, 0x0a, 0x02]).is_ok() {
            TransitionCause::Manual
        } else {
            TransitionCause::Schedule
        };

        byte_seq.assert_const(&[0xc6, 0x14])?;
        let modified_filetime = byte_seq
            .read_vlq_64()?
            .try_into()
            .map_err(|_| ParseError::ValueNotInRange)?;
        if modified_filetime > LATEST_FILETIME {
            return Err(ParseError::ValueNotInRange);
        }

        (0..4)
            .try_for_each(|_| byte_seq.assert_zero())
            .and_then(|_| byte_seq.assert_exhausted())
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;

        Ok(Self {
            prologue_epoch_secs,
            active,
            transition_cause,
            modified_filetime,
        })
    }

    pub fn lenient_fallback(now: SystemTime) -> Self {
        let epoch_duration = system_time_to_epoch_duration(now);
        Self {
            prologue_epoch_secs: epoch_duration_to_epoch_secs(epoch_duration),
            active: TrackedValue::new(false),
            transition_cause: TransitionCause::Manual,
            modified_filetime: epoch_duration_to_filetime(epoch_duration),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let now_epoch_duration = now_as_epoch_duration();
        let now_epoch_secs = epoch_duration_to_epoch_secs(now_epoch_duration);
        let now_filetime = epoch_duration_to_filetime(now_epoch_duration);

        const MAX_BODY_LEN: usize = 21;
        let mut body_byte_seq = ByteSeq::with_capacity(MAX_BODY_LEN);

        body_byte_seq.push_zero();
        if *self.active {
            body_byte_seq.push_const(&[0x10, 0x00]);
        }
        if self.transition_cause == TransitionCause::Manual {
            body_byte_seq.push_const(&[0xd0, 0x0a, 0x02]);
        }

        body_byte_seq.push_const(&[0xc6, 0x14]);
        body_byte_seq.push_vlq_64(now_filetime as _);

        for _ in 0..4 {
            body_byte_seq.push_zero();
        }

        let mut byte_seq = CloudStoreValuePrologue {
            epoch_secs: Some(now_epoch_secs.max(self.prologue_epoch_secs + 2)),
            num_body_bytes: Some(body_byte_seq.len() as _),
        }
        .to_byte_seq(Some(MAX_BODY_LEN));
        byte_seq.extend(&body_byte_seq);

        byte_seq.into()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TransitionCause {
    Manual,
    Schedule,
}

#[cfg(test)]
mod tests {
    use super::TransitionCause;
    use crate::{
        cloud_store::night_light::state::RawNightLightState,
        data_conversion::{
            time::{
                epoch_duration_to_epoch_secs, epoch_duration_to_filetime, now_as_epoch_duration,
            },
            Strictness, TrackedValue,
        },
    };

    #[test]
    fn shortest_from_and_to_bytes() {
        let bytes = [
            0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xae, 0x81, 0xd2, 0xa9,
            0x06, 0x2a, 0x2b, 0x0e, 0x10, 0x43, 0x42, 0x01, 0x00, 0xc6, 0x14, 0xe6, 0xfd, 0x92,
            0xd6, 0xa9, 0x91, 0x81, 0xed, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            match RawNightLightState::from_bytes(bytes.to_vec(), strictness) {
                Ok(state) => {
                    assert_eq!(*state.active, false);
                    assert_eq!(state.transition_cause, TransitionCause::Schedule);
                    assert_eq!(state.to_bytes().len(), bytes.len());
                }
                result => panic!("{result:?}"),
            }
        }
    }

    #[test]
    fn longest_from_and_to_bytes() {
        let bytes = [
            0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xae, 0x81, 0xd2, 0xa9,
            0x06, 0x2a, 0x2b, 0x0e, 0x15, 0x43, 0x42, 0x01, 0x00, 0x10, 0x00, 0xd0, 0x0a, 0x02,
            0xc6, 0x14, 0xe6, 0xfd, 0x92, 0xd6, 0xa9, 0x91, 0x81, 0xed, 0x01, 0x00, 0x00, 0x00,
            0x00,
        ];

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            match RawNightLightState::from_bytes(bytes.to_vec(), strictness) {
                Ok(state) => {
                    assert_eq!(*state.active, true);
                    assert_eq!(state.transition_cause, TransitionCause::Manual);
                    assert_eq!(state.to_bytes().len(), bytes.len());
                }
                result => panic!("{result:?}"),
            }
        }
    }

    #[test]
    fn check_serialized_len() {
        let now_epoch_duration = now_as_epoch_duration();
        let now_epoch_secs = epoch_duration_to_epoch_secs(now_epoch_duration);
        let now_filetime = epoch_duration_to_filetime(now_epoch_duration);

        let state = RawNightLightState {
            prologue_epoch_secs: now_epoch_secs,
            active: TrackedValue::new(true),
            transition_cause: TransitionCause::Manual,
            modified_filetime: now_filetime,
        };
        assert_eq!(state.to_bytes().len(), 43);

        let state = RawNightLightState {
            prologue_epoch_secs: now_epoch_secs,
            active: TrackedValue::new(false),
            transition_cause: TransitionCause::Manual,
            modified_filetime: now_filetime,
        };
        assert_eq!(state.to_bytes().len(), 41);

        let state = RawNightLightState {
            prologue_epoch_secs: now_epoch_secs,
            active: TrackedValue::new(false),
            transition_cause: TransitionCause::Schedule,
            modified_filetime: now_filetime,
        };
        assert_eq!(state.to_bytes().len(), 38);

        // (The timestamps won't make the byte count grow until at least the year 3000.)
    }
}
