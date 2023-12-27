use std::fmt::{self};

pub struct HexBytes<'a> {
    bytes: &'a [u8],
    old_bytes: Option<&'a [u8]>,
}

impl<'a> HexBytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            old_bytes: None,
        }
    }

    pub fn diff_against(mut self, old_bytes: &'a [u8]) -> Self {
        //! This will introduce ANSI escape sequences in the output to color it.

        self.old_bytes = Some(old_bytes);
        self
    }
}

impl fmt::Display for HexBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let write_byte = |f: &mut fmt::Formatter<'_>, first: bool, byte| -> fmt::Result {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{byte:02x}")?;
            Ok(())
        };

        let mut first = true;
        if let Some(other_bytes) = self.old_bytes {
            let mut current_color = Color::Default;
            write!(f, "{current_color}")?;

            for fragment in diff::slice(other_bytes, self.bytes) {
                let (byte, byte_color) = match fragment {
                    diff::Result::Left(byte) => (byte, Color::BrightRed),
                    diff::Result::Right(byte) => (byte, Color::BrightGreen),
                    diff::Result::Both(byte, _) => (byte, Color::Default),
                };

                if byte_color != current_color {
                    write!(f, "{byte_color}")?;
                    current_color = byte_color;
                }

                write_byte(f, first, byte)?;

                first = false;
            }

            if current_color != Color::Default {
                write!(f, "{}", Color::Default)?;
            }
        } else {
            for byte in self.bytes {
                write_byte(f, first, byte)?;
                first = false;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    Default,
    BrightRed,
    BrightGreen,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                // See <https://chrisyeh96.github.io/2020/03/28/terminal-colors.html>.
                Color::Default => "\x1b[0m",
                Color::BrightRed => "\x1b[91m",
                Color::BrightGreen => "\x1b[92m",
            }
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data_conversion::hex_bytes::HexBytes;

    #[test]
    fn diff_output() {
        assert_eq!(
            HexBytes::new(&[0x10, 0xf1, 0xf2, 0x13])
                .diff_against(&[0x10, 0x11, 0x12, 0x13])
                .to_string(),
            String::new()
                + "\x1b[0m"
                + "10"
                + "\x1b[91m"
                + " 11 12"
                + "\x1b[92m"
                + " f1 f2"
                + "\x1b[0m"
                + " 13"
        );
    }
}
