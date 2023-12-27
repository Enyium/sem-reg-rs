impl ByteSeq {
    //...

    pub fn read_ansi_string(&mut self, len: Option<usize>) -> Result<Vec<u8>, ParseError> {
        //! Reads a zero-terminated ANSI string.

        if let Some((ansi_str, size)) = self.get_ansi_str(self.read_index, len) {
            let ansi_string = ansi_str.to_vec();
            self.read_index += size;
            Ok(ansi_string)
        } else {
            Err(ParseError::ExpectedAnsiString(self.read_index))
        }
    }

    fn get_ansi_str(&self, start_index: usize, mode: StringMode) -> Option<(&[u8], usize)> {
        let mut end_index = start_index;

        loop {
            if let Some(byte) = self.bytes.get(end_index) {
                //                 if *byte == 0 {
                //                     break if len.is_some() {
                //                         None
                //                     } else {
                //                         let slice = &self.bytes[start_index..end_index];
                //                         Some((slice, slice.len() + 1))
                //                     };
                //                 }
                //
                //                 if Some(end_index - start_index) == len {
                //                     let slice = &self.bytes[start_index..end_index];
                //                     break Some((slice, slice.len()));
                //                 }

                match mode {
                    StringMode::Len(len) => {
                        if *byte == 0 {
                            break None;
                        }

                        if end_index - start_index == len {
                            break Some((&self.bytes[start_index..end_index], len));
                        }

                        end_index += 1;
                    }
                    StringMode::TillZero => {
                        if *byte == 0 {
                            let slice = &self.bytes[start_index..end_index];
                            break Some((slice, slice.len() + 1));
                        } else {
                            //...
                        }
                    }
                    StringMode::TillZeroInSectionLen(len) => {
                        if *byte == 0 {
                            //...
                        } else {
                            //...
                        }
                    }
                }

                // end_index += 1;
            } else {
                break None;
            }
        }
    }

    pub fn push_ansi_str(&mut self, ansi_str: &[u8]) {
        self.bytes.extend_from_slice(ansi_str);
        self.bytes.push(0);
    }

    pub fn read_wide_string(&mut self, len: Option<usize>) -> Result<Vec<u16>, ParseError> {
        //! Reads a wide string, zero-terminated if `len` is `None`.

        if let Some((wide_string, size)) = self.get_wide_string(self.read_index, len) {
            self.read_index += size;
            Ok(wide_string)
        } else {
            Err(ParseError::ExpectedWideString(self.read_index))
        }
    }

    fn get_wide_string(&self, start_index: usize, len: Option<usize>) -> Option<(Vec<u16>, usize)> {
        let mut wide_string = Vec::new();
        let mut byte_index = start_index;

        loop {
            if let (Some(byte_1), Some(byte_2)) =
                (self.bytes.get(byte_index), self.bytes.get(byte_index + 1))
            {
                let wide_char = u16::from_le_bytes([*byte_1, *byte_2]);

                if wide_char == 0 {
                    break if len.is_some() {
                        None
                    } else {
                        Some((wide_string, (wide_string.len() + 1) * 2))
                    };
                }

                wide_string.push(wide_char);

                if Some(wide_string.len()) == len {
                    break Some((wide_string, wide_string.len() * 2));
                }

                byte_index += 2;
            } else {
                break None;
            }
        }
    }

    pub fn push_wide_string(&mut self, wide_string: &[u16]) {
        // Data is already in little endian.
        self.bytes.extend_from_slice(unsafe {
            slice::from_raw_parts(wide_string.as_ptr() as *const u8, wide_string.len() * 2)
        });
        self.bytes.extend_from_slice(&[0, 0]);
    }

    pub fn find_data(&self, reduced_amount: bool) -> Vec<Datum> {
        //! Helper for finding out the format or registry values. Tries to interpret the data at every index in a variety of formats. Imperfect and not thoroughly tested.
        //!
        //! Strings may also be length-prefixed, where the length may be a VLQ with possibly just one byte.

        let mut data = Vec::new();

        for i in 0..self.bytes.len() {
            let u_16 = self.get_int::<u16>(i);
            let u_32 = self.get_int::<u32>(i);
            let u_64 = self.get_int::<u64>(i);

            let mut must_push_u_16 = false;
            let mut must_push_u_32 = false;
            let mut must_push_u_64 = false;

            let vlq_64 = self.get_vlq_64(i).filter(|(_, size)| *size >= 2);

            let ascii_string = self
                .get_ansi_str(i)
                .filter(|(ansi_str, _)| {
                    ansi_str.len() >= 4 && ansi_str.iter().all(|&byte| byte >= 0x20 && byte < 0x80)
                })
                .map(|(ansi_str, size)| {
                    (
                        unsafe { str::from_utf8_unchecked(ansi_str) }.to_string(),
                        size,
                    )
                });
            let wide_string = self
                .get_wide_string(i)
                .filter(|(wide_string, _)| {
                    wide_string.len() >= 4
                        && wide_string
                            .iter()
                            .all(|&wide_char| wide_char >= 0x20 && wide_char < 0x2600)
                })
                .and_then(|(wide_string, size)| {
                    String::from_utf16(&wide_string)
                        .ok()
                        .map(|string| (string, size))
                });

            // Byte count.
            let bytes_left = self.bytes.len() - i;

            let mut byte_count = None;

            let mut set_byte_count =
                |int: Option<(usize, usize)>, extra_action: &mut dyn FnMut()| {
                    if let (None, Some((int, size))) = (byte_count, int) {
                        let bytes_left_all = bytes_left - size;
                        let bytes_left_less = bytes_left_all.saturating_sub(10);
                        byte_count = (int > 0 && int >= bytes_left_less && int <= bytes_left_all)
                            .then(|| {
                                extra_action();
                                int as _
                            });
                    }
                };

            set_byte_count(vlq_64.map(|(value, size)| (value as _, size)), &mut || {});
            set_byte_count(u_16.map(|(value, size)| (value as _, size)), &mut || {
                must_push_u_16 = true;
            });
            set_byte_count(u_32.map(|(value, size)| (value as _, size)), &mut || {
                must_push_u_32 = true;
            });

            // Timestamp.
            const MIN_EPOCH_SECS: u32 = 1420070400; // 2015-01-01T00:00Z
            let max_epoch_secs = chrono::Utc::now().timestamp() + 31536000; // Now plus one year.
            const MIN_FILETIME: u64 = 130645440000000000; // 2015-01-01T00:00Z
            let max_filetime = FileTime::now().filetime() as u64 + 315360000000000; // Now plus one year.

            let epoch_secs_from_u_32 = u_32.map(|(value, _)| value as u64).filter(|value| {
                *value >= MIN_EPOCH_SECS as _ && *value <= max_epoch_secs as _ && !reduced_amount
            });
            must_push_u_32 = must_push_u_32 || epoch_secs_from_u_32.is_some();

            let filetime_from_u_64 = u_64
                .map(|(value, _)| value)
                .filter(|value| *value >= MIN_FILETIME && *value <= max_filetime);
            must_push_u_64 = must_push_u_64 || filetime_from_u_64.is_some();

            let epoch_secs_from_vlq = vlq_64
                .map(|(value, _)| value)
                .filter(|value| *value >= MIN_EPOCH_SECS as _ && *value <= max_epoch_secs as _);
            let filetime_from_vlq = vlq_64
                .map(|(value, _)| value)
                .filter(|value| *value >= MIN_FILETIME && *value <= max_filetime);

            // Return data.
            if let Some(byte_count) = byte_count {
                data.push(Datum::ByteCount(byte_count, i));
            }

            for epoch_secs in [epoch_secs_from_u_32, epoch_secs_from_vlq] {
                if let Some(epoch_secs) = epoch_secs {
                    data.push(Datum::EpochSecsTimestamp(
                        utc_epoch_secs_to_local_iso_string(epoch_secs),
                        i,
                    ));
                }
            }

            for filetime in [filetime_from_u_64, filetime_from_vlq] {
                if let Some(filetime) = filetime {
                    data.push(Datum::FiletimeTimestamp(
                        utc_filetime_to_local_iso_string(filetime),
                        i,
                    ));
                }
            }

            if let (true, Some((value, _))) = (must_push_u_16, u_16) {
                data.push(Datum::U16(value, i));
            }

            if let (true, Some((value, _))) = (must_push_u_32, u_32) {
                data.push(Datum::U32(value, i));
            }

            if let (true, Some((value, _))) = (must_push_u_64, u_64) {
                data.push(Datum::U64(value, i));
            }

            if let Some((value, _)) = vlq_64 {
                data.push(Datum::Vlq64(value, Self::zigzag_64_decode(value as _), i));
            }

            if let Some((ascii_string, _)) = ascii_string {
                data.push(Datum::AsciiString(ascii_string, i));
            }

            if let Some((wide_string, _)) = wide_string {
                let mut must_push = true;
                for datum in data.iter() {
                    if let Datum::Utf16WideString(other_wide_string, _) = datum {
                        if other_wide_string.ends_with(&wide_string) {
                            must_push = false;
                        }
                    }
                }

                if must_push {
                    data.push(Datum::Utf16WideString(wide_string, i));
                }
            }
        }

        data
    }
}

pub enum StringMode {
    Len(usize),
    TillZero,
    TillZeroInSectionLen(usize),
}

/// A possible interpretation of raw bytes. All variants have the index at which the datum was found as their first tuple item.
#[derive(Debug)]
pub enum Datum {
    /// A number that also has another reasonable interpretation.
    U16(u16, usize),
    /// A number that also has another reasonable interpretation.
    U32(u32, usize),
    /// A number that also has another reasonable interpretation.
    U64(u64, usize),
    /// VLQ with at least 2 bytes. Additionally presented in zigzag-decoded form.
    Vlq64(u64, i64, usize),
    /// Possible number of bytes between the datum and the end.
    ByteCount(usize, usize),
    /// Only between years 2000 and 2100.
    EpochSecsTimestamp(String, usize),
    /// Only between years 2000 and 2100.
    FiletimeTimestamp(String, usize),
    /// Only code points > U+0020.
    AsciiString(String, usize),
    /// Only code points > U+0020.
    Utf16WideString(String, usize),
}

#[cfg(test)]
mod tests {
    use super::ByteSeq;

    #[ignore]
    #[test]
    fn tmp() {
        let byte_seq = ByteSeq::from_bytes(
            [
                0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xb9, 0xdd, 0xae, 0xaa,
                0x06, 0x2a, 0x2b, 0x0e, 0xa1, 0x04, 0x43, 0x42, 0x01, 0x00, 0x12, 0x60, 0x7b, 0x00,
                0x39, 0x00, 0x30, 0x00, 0x31, 0x00, 0x32, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00,
                0x30, 0x00, 0x2d, 0x00, 0x30, 0x00, 0x30, 0x00, 0x31, 0x00, 0x37, 0x00, 0x2d, 0x00,
                0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x2d, 0x00, 0x30, 0x00, 0x30, 0x00,
                0x30, 0x00, 0x30, 0x00, 0x2d, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00,
                0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x46, 0x00, 0x46, 0x00, 0x31, 0x00, 0x43, 0x00,
                0x45, 0x00, 0x7d, 0x00, 0x5f, 0x00, 0x53, 0x00, 0x68, 0x00, 0x61, 0x00, 0x72, 0x00,
                0x65, 0x00, 0x50, 0x00, 0x6f, 0x00, 0x69, 0x00, 0x6e, 0x00, 0x74, 0x00, 0x44, 0x00,
                0x65, 0x00, 0x73, 0x00, 0x69, 0x00, 0x67, 0x00, 0x6e, 0x00, 0x65, 0x00, 0x72, 0x00,
                0x5f, 0x00, 0x7b, 0x00, 0x30, 0x00, 0x32, 0x00, 0x32, 0x00, 0x30, 0x00, 0x36, 0x00,
                0x44, 0x00, 0x43, 0x00, 0x43, 0x00, 0x2d, 0x00, 0x30, 0x00, 0x43, 0x00, 0x41, 0x00,
                0x46, 0x00, 0x2d, 0x00, 0x34, 0x00, 0x36, 0x00, 0x42, 0x00, 0x42, 0x00, 0x2d, 0x00,
                0x38, 0x00, 0x45, 0x00, 0x44, 0x00, 0x43, 0x00, 0x2d, 0x00, 0x36, 0x00, 0x43, 0x00,
                0x32, 0x00, 0x38, 0x00, 0x31, 0x00, 0x41, 0x00, 0x41, 0x00, 0x32, 0x00, 0x31, 0x00,
                0x45, 0x00, 0x46, 0x00, 0x41, 0x00, 0x7d, 0x00, 0x32, 0x0c, 0x45, 0x00, 0x78, 0x00,
                0x74, 0x00, 0x65, 0x00, 0x72, 0x00, 0x6e, 0x00, 0x61, 0x00, 0x6c, 0x00, 0x20, 0x00,
                0x4d, 0x00, 0x53, 0x00, 0x49, 0x00, 0x4a, 0x00, 0x6a, 0x00, 0x92, 0x42, 0x55, 0x00,
                0x70, 0x00, 0x64, 0x00, 0x61, 0x00, 0x74, 0x00, 0x65, 0x00, 0x20, 0x00, 0x66, 0x00,
                0x6f, 0x00, 0x72, 0x00, 0x20, 0x00, 0x4d, 0x00, 0x69, 0x00, 0x63, 0x00, 0x72, 0x00,
                0x6f, 0x00, 0x73, 0x00, 0x6f, 0x00, 0x66, 0x00, 0x74, 0x00, 0x20, 0x00, 0x4f, 0x00,
                0x66, 0x00, 0x66, 0x00, 0x69, 0x00, 0x63, 0x00, 0x65, 0x00, 0x20, 0x00, 0x32, 0x00,
                0x30, 0x00, 0x30, 0x00, 0x37, 0x00, 0x20, 0x00, 0x73, 0x00, 0x75, 0x00, 0x69, 0x00,
                0x74, 0x00, 0x65, 0x00, 0x73, 0x00, 0x20, 0x00, 0x28, 0x00, 0x4b, 0x00, 0x42, 0x00,
                0x32, 0x00, 0x35, 0x00, 0x39, 0x00, 0x36, 0x00, 0x37, 0x00, 0x38, 0x00, 0x37, 0x00,
                0x29, 0x00, 0x20, 0x00, 0x33, 0x00, 0x32, 0x00, 0x2d, 0x00, 0x42, 0x00, 0x69, 0x00,
                0x74, 0x00, 0x20, 0x00, 0x45, 0x00, 0x64, 0x00, 0x69, 0x00, 0x74, 0x00, 0x69, 0x00,
                0x6f, 0x00, 0x6e, 0x00, 0xb2, 0x09, 0x4d, 0x00, 0x69, 0x00, 0x63, 0x00, 0x72, 0x00,
                0x6f, 0x00, 0x73, 0x00, 0x6f, 0x00, 0x66, 0x00, 0x74, 0x00, 0xd2, 0x0a, 0x27, 0x68,
                0x00, 0x74, 0x00, 0x74, 0x00, 0x70, 0x00, 0x3a, 0x00, 0x2f, 0x00, 0x2f, 0x00, 0x73,
                0x00, 0x75, 0x00, 0x70, 0x00, 0x70, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x74, 0x00, 0x2e,
                0x00, 0x6d, 0x00, 0x69, 0x00, 0x63, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x73, 0x00, 0x6f,
                0x00, 0x66, 0x00, 0x74, 0x00, 0x2e, 0x00, 0x63, 0x00, 0x6f, 0x00, 0x6d, 0x00, 0x2f,
                0x00, 0x6b, 0x00, 0x62, 0x00, 0x2f, 0x00, 0x32, 0x00, 0x35, 0x00, 0x39, 0x00, 0x36,
                0x00, 0x37, 0x00, 0x38, 0x00, 0x37, 0x00, 0xd2, 0x0b, 0x27, 0x68, 0x00, 0x74, 0x00,
                0x74, 0x00, 0x70, 0x00, 0x3a, 0x00, 0x2f, 0x00, 0x2f, 0x00, 0x73, 0x00, 0x75, 0x00,
                0x70, 0x00, 0x70, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x74, 0x00, 0x2e, 0x00, 0x6d, 0x00,
                0x69, 0x00, 0x63, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x73, 0x00, 0x6f, 0x00, 0x66, 0x00,
                0x74, 0x00, 0x2e, 0x00, 0x63, 0x00, 0x6f, 0x00, 0x6d, 0x00, 0x2f, 0x00, 0x6b, 0x00,
                0x62, 0x00, 0x2f, 0x00, 0x32, 0x00, 0x35, 0x00, 0x39, 0x00, 0x36, 0x00, 0x37, 0x00,
                0x38, 0x00, 0x37, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
            .to_vec(),
        );
        // Same data: 43 42 01 00 0a 02 01 00 2a 06 b9 dd ae aa 06 2a 2b 0e a1 04 43 42 01 00 12 60 7b 00 39 00 30 00 31 00 32 00 30 00 30 00 30 00 30 00 2d 00 30 00 30 00 31 00 37 00 2d 00 30 00 30 00 30 00 30 00 2d 00 30 00 30 00 30 00 30 00 2d 00 30 00 30 00 30 00 30 00 30 00 30 00 30 00 46 00 46 00 31 00 43 00 45 00 7d 00 5f 00 53 00 68 00 61 00 72 00 65 00 50 00 6f 00 69 00 6e 00 74 00 44 00 65 00 73 00 69 00 67 00 6e 00 65 00 72 00 5f 00 7b 00 30 00 32 00 32 00 30 00 36 00 44 00 43 00 43 00 2d 00 30 00 43 00 41 00 46 00 2d 00 34 00 36 00 42 00 42 00 2d 00 38 00 45 00 44 00 43 00 2d 00 36 00 43 00 32 00 38 00 31 00 41 00 41 00 32 00 31 00 45 00 46 00 41 00 7d 00 32 0c 45 00 78 00 74 00 65 00 72 00 6e 00 61 00 6c 00 20 00 4d 00 53 00 49 00 4a 00 6a 00 92 42 55 00 70 00 64 00 61 00 74 00 65 00 20 00 66 00 6f 00 72 00 20 00 4d 00 69 00 63 00 72 00 6f 00 73 00 6f 00 66 00 74 00 20 00 4f 00 66 00 66 00 69 00 63 00 65 00 20 00 32 00 30 00 30 00 37 00 20 00 73 00 75 00 69 00 74 00 65 00 73 00 20 00 28 00 4b 00 42 00 32 00 35 00 39 00 36 00 37 00 38 00 37 00 29 00 20 00 33 00 32 00 2d 00 42 00 69 00 74 00 20 00 45 00 64 00 69 00 74 00 69 00 6f 00 6e 00 b2 09 4d 00 69 00 63 00 72 00 6f 00 73 00 6f 00 66 00 74 00 d2 0a 27 68 00 74 00 74 00 70 00 3a 00 2f 00 2f 00 73 00 75 00 70 00 70 00 6f 00 72 00 74 00 2e 00 6d 00 69 00 63 00 72 00 6f 00 73 00 6f 00 66 00 74 00 2e 00 63 00 6f 00 6d 00 2f 00 6b 00 62 00 2f 00 32 00 35 00 39 00 36 00 37 00 38 00 37 00 d2 0b 27 68 00 74 00 74 00 70 00 3a 00 2f 00 2f 00 73 00 75 00 70 00 70 00 6f 00 72 00 74 00 2e 00 6d 00 69 00 63 00 72 00 6f 00 73 00 6f 00 66 00 74 00 2e 00 63 00 6f 00 6d 00 2f 00 6b 00 62 00 2f 00 32 00 35 00 39 00 36 00 37 00 38 00 37 00 00 00 00 00

        for datum in byte_seq.find_data(true) {
            println!("{:?}", datum);
        }
    }

    //...
}

pub enum ParseError {
    //...
    /// Expected a null-terminated ANSI or ASCII string with 1 byte per character.
    #[error("expected an ANSI string at byte index {0}")]
    ExpectedAnsiString(usize),
    /// Expected a null-terminated wide string with 2 bytes per character (little endian).
    #[error("expected a wide string at byte index {0}")]
    ExpectedWideString(usize),
    //...
}
