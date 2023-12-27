use std::mem;
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug)]
pub struct ByteSeq {
    bytes: Vec<u8>,
    read_index: usize,
}

impl ByteSeq {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            read_index: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
            read_index: 0,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            read_index: 0,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn read_index(&self) -> usize {
        self.read_index
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn num_bytes_left(&self) -> usize {
        self.bytes.len() - self.read_index
    }

    pub fn seek(&mut self, index: usize) -> bool {
        if index <= self.bytes.len() {
            self.read_index = index;
            true
        } else {
            false
        }
    }

    pub fn seek_by(&mut self, num_bytes: usize) -> bool {
        self.seek(self.read_index + num_bytes)
    }

    pub fn assert_const(&mut self, r#const: &[u8]) -> Result<(), ParseError> {
        self.bytes[self.read_index..]
            .starts_with(r#const)
            .then(|| {
                self.read_index += r#const.len();
                ()
            })
            .ok_or(ParseError::ExpectedConst(self.read_index))
    }

    pub fn push_const(&mut self, r#const: &[u8]) {
        self.bytes.extend_from_slice(r#const);
    }

    pub fn assert_zero(&mut self) -> Result<(), ParseError> {
        self.bytes[self.read_index..]
            .starts_with(&[0x00])
            .then(|| {
                self.read_index += 1;
                ()
            })
            .ok_or(ParseError::ExpectedZero(self.read_index))
    }

    pub fn push_zero(&mut self) {
        self.bytes.push(0);
    }

    pub fn read_int<T: FromBytes>(&mut self) -> Result<T, ParseError> {
        //! This and the respective push-function can just use the native byte order, because this code is just for Windows, which always uses little endian, and registry values should also always use little endian.

        if let Some((value, size)) = self.get_int(self.read_index) {
            self.read_index += size;
            Ok(value)
        } else {
            Err(ParseError::ExpectedInt(self.read_index))
        }
    }

    fn get_int<T: FromBytes>(&self, index: usize) -> Option<(T, usize)> {
        T::read_from_prefix(&self.bytes[index..]).map(|value| (value, mem::size_of::<T>()))
    }

    pub fn push_int<T: AsBytes>(&mut self, int: T) {
        self.bytes.extend_from_slice(T::as_bytes(&int));
    }

    pub fn read_vlq_64(&mut self) -> Result<u64, ParseError> {
        if let Some((value, size)) = self.get_vlq_64(self.read_index) {
            self.read_index += size;
            Ok(value)
        } else {
            Err(ParseError::ExpectedVlq64(self.read_index))
        }
    }

    fn get_vlq_64(&self, start_index: usize) -> Option<(u64, usize)> {
        let mut value = 0;

        let mut index = start_index;
        let mut shift = 0;

        loop {
            let byte = if let Some(byte) = self.bytes.get(index) {
                byte
            } else {
                // No concluding byte - not VLQ.
                break None;
            };

            if shift == 63 && byte & 0b1111_1110 != 0 {
                // Bits other than LSB would result in overflow. This also rules out even more loop iterations, because it ensures that the MSB is zero, which leads to the loop successfully breaking below.
                break None;
            }

            value += ((byte & 0b0111_1111) as u64) << shift;

            index += 1;
            shift += 7;

            if byte & 0b1000_0000 == 0 {
                // Concluding byte - done.
                break Some((value, index - start_index));
            }
        }
    }

    pub fn push_vlq_64(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0b0111_1111) as u8;
            value >>= 7;
            if value != 0 {
                // One or more bytes will follow. Set continuation bit.
                byte |= 0b1000_0000;
            }
            self.bytes.push(byte);

            if value == 0 {
                break;
            }
        }
    }

    pub fn read_zigzag_vlq_64(&mut self) -> Result<i64, ParseError> {
        if let Some((value, size)) = self.get_zigzag_vlq_64(self.read_index) {
            self.read_index += size;
            Ok(value)
        } else {
            Err(ParseError::ExpectedVlq64(self.read_index))
        }
    }

    fn get_zigzag_vlq_64(&self, start_index: usize) -> Option<(i64, usize)> {
        self.get_vlq_64(start_index)
            .map(|(value, size)| (Self::zigzag_64_decode(value), size))
    }

    pub fn push_zigzag_vlq_64(&mut self, value: i64) {
        self.push_vlq_64(Self::zigzag_64_encode(value));
    }

    fn zigzag_64_decode(encoded: u64) -> i64 {
        //! Performs zigzag decoding on an unsigned integer to retrieve the original signed integer.

        // Get rid of sign bit and correct placement of data bits (shifting unsigned data type inserts zeroes).
        let data_bits = (encoded >> 1) as i64;

        // Negate data bits if sign bit (LSB) is set.
        if encoded & 1 != 0 {
            !data_bits
        } else {
            data_bits
        }
    }

    fn zigzag_64_encode(value: i64) -> u64 {
        // Shift data bits by 1 to swap them with sign bit - negated, if negative. Then potentially add an LSB sign bit.
        (if value >= 0 {
            value << 1
        } else {
            !value << 1 | 1
        }) as u64
    }

    pub fn exhausted(&self) -> bool {
        self.read_index >= self.bytes.len()
    }

    pub fn assert_exhausted(&self) -> Result<(), ParseError> {
        self.exhausted()
            .then_some(())
            .ok_or(ParseError::DataAfterExpectedEnd)
    }

    pub fn extend(&mut self, other: &Self) {
        self.bytes.extend_from_slice(&other.bytes);
    }
}

impl From<ByteSeq> for Vec<u8> {
    fn from(value: ByteSeq) -> Self {
        value.bytes
    }
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum ParseError {
    /// Expected certain bytes. This and some other variants bring the byte index with it where the respective item was expected, but not found.
    #[error("expected one or more constant bytes at index {0}")]
    ExpectedConst(usize),
    /// Expected a zero-byte.
    #[error("expected a zero byte at index {0}")]
    ExpectedZero(usize),
    /// Expected an integer with a certain byte size (little endian).
    #[error("expected a fixed-width integer at byte index {0}")]
    ExpectedInt(usize),
    /// Expected a VLQ (variable-length quantity) with a maximum of 64 data bits (little endian; possibly also zigzag-encoded).
    #[error("expected a variable-length quantity at byte index {0}")]
    ExpectedVlq64(usize),
    /// Encountered an exceptional value.
    #[error("value not in expected range")]
    ValueNotInRange,
    /// Different parts of the data don't harmonize with each other.
    #[error("parts of data inconsistent with each other")]
    InconsistentData,
    /// Expected the end of the byte stream, but still found data.
    #[error("expected end of byte stream, got more data")]
    DataAfterExpectedEnd,
}
