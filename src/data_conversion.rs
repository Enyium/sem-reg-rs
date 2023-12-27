pub(crate) mod byte_seq;
pub(crate) mod format;
pub mod hex_bytes;
pub(crate) mod time;

pub use byte_seq::ParseError;

use std::ops::Deref;

#[derive(Debug)]
pub struct TrackedValue<T: PartialEq> {
    old: T,
    new: Option<T>,
}

impl<T: PartialEq> TrackedValue<T> {
    pub fn new(value: T) -> Self {
        Self {
            old: value,
            new: None,
        }
    }

    pub fn set(&mut self, value: T) {
        self.new = Some(value);
    }

    pub fn reset(&mut self) {
        self.new = None;
    }

    pub fn changed(&self) -> bool {
        if let Some(new) = &self.new {
            *new != self.old
        } else {
            false
        }
    }
}

impl<T: PartialEq> Deref for TrackedValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.new.as_ref().unwrap_or(&self.old)
    }
}

impl<T: PartialEq> PartialEq for TrackedValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.new
            .as_ref()
            .unwrap_or(&self.old)
            .eq(other.new.as_ref().unwrap_or(&other.old))
    }
}

/// The mode of operation when parsing and checking values for validity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Strictness {
    /// Only accept known formats exact to the byte, and only value ranges last known to be valid. Should normally be used.
    Strict,
    /// Turn a blind eye regarding certain aspects when parsing or checking values for validity. Can be tried when strict mode doesn't work, or when you must overcome certain limitations. Doesn't influence serialization. Rather than this being used regularly, the parsing or other code should be updated.
    Lenient,
}

impl Strictness {
    pub fn from_lenient_bool(lenient: bool) -> Self {
        if lenient {
            Self::Lenient
        } else {
            Self::Strict
        }
    }

    pub fn is_strict(&self) -> bool {
        *self == Strictness::Strict
    }

    pub fn is_lenient(&self) -> bool {
        *self == Strictness::Lenient
    }
}

pub trait ResultOrElseIf<T, E> {
    fn or_else_if<O>(self, flag: bool, op: O) -> Result<T, E>
    where
        O: FnOnce(E) -> Result<T, E>;
}

impl<T, E> ResultOrElseIf<T, E> for Result<T, E> {
    fn or_else_if<O>(self, flag: bool, op: O) -> Result<T, E>
    where
        O: FnOnce(E) -> Result<T, E>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                if flag {
                    op(e)
                } else {
                    Err(e)
                }
            }
        }
    }
}
