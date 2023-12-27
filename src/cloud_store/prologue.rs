use crate::data_conversion::{
    byte_seq::{ByteSeq, ParseError},
    ResultOrElseIf, Strictness,
};

#[derive(PartialEq, Debug)]
pub struct CloudStoreValuePrologue {
    /// The Unix timestamp when the setting was last set. From what can be observed from the Night Light registry values, when writing a registry value, this number should always be greater than the number in the current registry value; otherwise, the registry value will be reverted. Windows sets this number in the Night Light registry values to the current time, or two seconds greater than the current number, whichever is greater (as of Nov. 2023).
    pub epoch_secs: Option<u32>,
    /// The number of bytes following the prologue.
    pub num_body_bytes: Option<u32>,
}

impl CloudStoreValuePrologue {
    // Examples for registry values (see tests for locations):
    // 43 42 01 00 0a 02 01 00 2a 2a -- -- -- -- -- -- -- -- -- -- -- -- 00 00 00 00
    // 43 42 01 00 0a -- -- 00 26 -- 88 e2 be a9 06 -- -- -- -- -- -- -- 00
    // 43 42 01 00 0a 02 01 00 2a 06 a0 b8 db aa 06 2a 2b 0e 20 43 42 01 ...   (Night Light state)
    //                               ||||||||||||||                   ^^ ^^- prefixed num body bytes
    //                               ^^^^^^^^^^^^^^- VLQ-encoded epoch secs

    pub fn from_byte_seq(
        byte_seq: &mut ByteSeq,
        strictness: Strictness,
    ) -> Result<Self, ParseError> {
        byte_seq.assert_const(&[0x43, 0x42, 0x01])?;

        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        byte_seq.assert_const(&[0x0a])?;
        let has_bytes_02_01 = byte_seq.assert_const(&[0x02, 0x01]).is_ok();

        byte_seq
            .assert_zero()
            .or_else_if(strictness.is_lenient(), |_| Ok(()))?;
        let (has_bytes_2a_2a, has_byte_26, has_bytes_2a_06) =
            if byte_seq.assert_const(&[0x2a, 0x2a]).is_ok() {
                (true, false, false)
            } else if byte_seq.assert_const(&[0x26]).is_ok() {
                (false, true, false)
            } else {
                byte_seq.assert_const(&[0x2a, 0x06])?;
                (false, false, true)
            };
        let epoch_secs = if has_bytes_2a_2a {
            None
        } else {
            Some(byte_seq.read_vlq_64()? as _)
        };

        let num_body_bytes = if has_bytes_2a_2a {
            if strictness.is_strict() && !has_bytes_02_01 {
                return Err(ParseError::InconsistentData);
            }

            (0..4)
                .try_for_each(|_| byte_seq.assert_zero())
                .and_then(|_| byte_seq.assert_exhausted())
                .or_else_if(strictness.is_lenient(), |_| Ok(()))?;

            None
        } else if has_byte_26 {
            if strictness.is_strict() && has_bytes_02_01 {
                return Err(ParseError::InconsistentData);
            }

            byte_seq
                .assert_zero()
                .and_then(|_| byte_seq.assert_exhausted())
                .or_else_if(strictness.is_lenient(), |_| Ok(()))?;

            None
        } else if has_bytes_2a_06 {
            if strictness.is_strict() && !has_bytes_02_01 {
                return Err(ParseError::InconsistentData);
            }

            byte_seq.assert_const(&[0x2a, 0x2b])?;
            byte_seq.assert_const(&[0x0e])?;
            let num_body_bytes = byte_seq
                .read_vlq_64()?
                .try_into()
                .map_err(|_| ParseError::ValueNotInRange)?;

            byte_seq
                .assert_const(&[0x43, 0x42, 0x01])
                .or_else_if(strictness.is_lenient(), |_| Ok(()))?;

            if strictness.is_strict() && byte_seq.num_bytes_left() != num_body_bytes as usize {
                return Err(ParseError::InconsistentData);
            }

            // (Body follows, but is parsed in more specific implementations.)

            Some(num_body_bytes)
        } else {
            unreachable!();
        };

        Ok(Self {
            epoch_secs,
            num_body_bytes,
        })
    }

    pub fn to_byte_seq(&self, additional_capacity: Option<usize>) -> ByteSeq {
        const MAX_PROLOGUE_LEN: usize = 22;
        let mut byte_seq =
            ByteSeq::with_capacity(if let Some(additional_capacity) = additional_capacity {
                MAX_PROLOGUE_LEN + additional_capacity
            } else {
                MAX_PROLOGUE_LEN
            });

        byte_seq.push_const(&[0x43, 0x42, 0x01]);

        byte_seq.push_zero();
        byte_seq.push_const(&[0x0a]);
        if self.epoch_secs.is_none() || self.num_body_bytes.is_some() {
            byte_seq.push_const(&[0x02, 0x01]);
        }

        byte_seq.push_zero();
        match self.epoch_secs {
            None => {
                byte_seq.push_const(&[0x2a, 0x2a]);

                for _ in 0..4 {
                    byte_seq.push_zero();
                }
            }
            Some(epoch_secs) => {
                match self.num_body_bytes {
                    None => byte_seq.push_const(&[0x26]),
                    Some(_) => byte_seq.push_const(&[0x2a, 0x06]),
                }
                byte_seq.push_vlq_64(epoch_secs as _);

                match self.num_body_bytes {
                    None => byte_seq.push_zero(),
                    Some(num_body_bytes) => {
                        byte_seq.push_const(&[0x2a, 0x2b]);
                        byte_seq.push_const(&[0x0e]);
                        byte_seq.push_vlq_64(num_body_bytes as _);

                        byte_seq.push_const(&[0x43, 0x42, 0x01]);
                    }
                }
            }
        }

        byte_seq
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cloud_store::prologue::CloudStoreValuePrologue,
        data_conversion::{
            byte_seq::{ByteSeq, ParseError},
            Strictness,
        },
    };

    /// Bodyless value `HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.globalization.culture.culturesettings\windows.data.globalization.culture.culturesettings\Data`. A few other registry values have exactly the same bytes.
    const VALUE_WITH_BYTES_2A_2A: [u8; 14] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x2a, 0x00, 0x00, 0x00, 0x00,
    ];
    const VALUE_WITH_BYTES_2A_2A_RESULT: Result<CloudStoreValuePrologue, ParseError> =
        Ok(CloudStoreValuePrologue {
            epoch_secs: None,
            num_body_bytes: None,
        });

    /// Bodyless value `HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Cloud\default$windows.data.controlcenter.uistate\windows.data.controlcenter.uistate\Data`.
    const VALUE_WITH_BYTE_26: [u8; 13] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x00, 0x26, 0x88, 0xe2, 0xbe, 0xa9, 0x06, 0x00,
    ];
    const VALUE_WITH_BYTE_26_RESULT: Result<CloudStoreValuePrologue, ParseError> =
        Ok(CloudStoreValuePrologue {
            epoch_secs: Some(1697624328),
            num_body_bytes: None,
        });

    /// Artificial Night Light state value with 0 body bytes.
    const NIGHT_LIGHT_STATE_VALUE: [u8; 22] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xa0, 0xb8, 0xdb, 0xaa, 0x06,
        0x2a, 0x2b, 0x0e, 0x00, 0x43, 0x42, 0x01,
    ];
    const NIGHT_LIGHT_STATE_VALUE_RESULT: Result<CloudStoreValuePrologue, ParseError> =
        Ok(CloudStoreValuePrologue {
            epoch_secs: Some(1700191264),
            num_body_bytes: Some(0),
        });

    fn parse_value_with_bytes_2a_2a(
        strictness: Strictness,
    ) -> Result<CloudStoreValuePrologue, ParseError> {
        CloudStoreValuePrologue::from_byte_seq(
            &mut ByteSeq::from_bytes(VALUE_WITH_BYTES_2A_2A.to_vec()),
            strictness,
        )
    }

    fn parse_value_with_byte_26(
        strictness: Strictness,
    ) -> Result<CloudStoreValuePrologue, ParseError> {
        CloudStoreValuePrologue::from_byte_seq(
            &mut ByteSeq::from_bytes(VALUE_WITH_BYTE_26.to_vec()),
            strictness,
        )
    }

    fn parse_night_light_state_value(
        strictness: Strictness,
    ) -> Result<CloudStoreValuePrologue, ParseError> {
        CloudStoreValuePrologue::from_byte_seq(
            &mut ByteSeq::from_bytes(NIGHT_LIGHT_STATE_VALUE.to_vec()),
            strictness,
        )
    }

    #[test]
    fn from_bytes_strict() {
        let result = parse_value_with_bytes_2a_2a(Strictness::Strict);
        assert_eq!(result, VALUE_WITH_BYTES_2A_2A_RESULT, "{result:?}");

        let result = parse_value_with_byte_26(Strictness::Strict);
        assert_eq!(result, VALUE_WITH_BYTE_26_RESULT, "{result:?}");

        let result = parse_night_light_state_value(Strictness::Strict);
        assert_eq!(result, NIGHT_LIGHT_STATE_VALUE_RESULT, "{result:?}");
    }

    #[test]
    fn from_bytes_lenient() {
        let result = parse_value_with_bytes_2a_2a(Strictness::Lenient);
        assert_eq!(result, VALUE_WITH_BYTES_2A_2A_RESULT, "{result:?}");

        let result = parse_value_with_byte_26(Strictness::Lenient);
        assert_eq!(result, VALUE_WITH_BYTE_26_RESULT, "{result:?}");

        let result = parse_night_light_state_value(Strictness::Lenient);
        assert_eq!(result, NIGHT_LIGHT_STATE_VALUE_RESULT, "{result:?}");
    }

    #[test]
    fn compare_strict_with_lenient_result() {
        let strict_result = parse_value_with_bytes_2a_2a(Strictness::Strict);
        let lenient_result = parse_value_with_bytes_2a_2a(Strictness::Lenient);
        assert_eq!(strict_result, lenient_result);

        let strict_result = parse_value_with_byte_26(Strictness::Strict);
        let lenient_result = parse_value_with_byte_26(Strictness::Lenient);
        assert_eq!(strict_result, lenient_result);

        let strict_result = parse_night_light_state_value(Strictness::Strict);
        let lenient_result = parse_night_light_state_value(Strictness::Lenient);
        assert_eq!(strict_result, lenient_result);
    }
}
