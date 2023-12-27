pub mod monitor;

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::{
    fs::File,
    io::{self},
    mem::ManuallyDrop,
    path::Path,
};
use winreg::{
    enums::{RegType, KEY_QUERY_VALUE, KEY_SET_VALUE},
    RegKey, RegValue, HKEY,
};

pub struct RegValuePath<'a> {
    pub hkey: HKEY,
    pub subkey_path: &'a str,
    pub value_name: &'a str,
}

pub fn read_reg_bin_value(reg_value_path: &RegValuePath) -> Result<Vec<u8>, io::Error> {
    let key = RegKey::predef(reg_value_path.hkey)
        .open_subkey_with_flags(reg_value_path.subkey_path, KEY_QUERY_VALUE)?;
    let value = key.get_raw_value(reg_value_path.value_name)?;

    if value.vtype == RegType::REG_BINARY {
        Ok(value.bytes)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected binary value",
        ))
    }
}

pub fn write_reg_bin_value(
    reg_value_path: &RegValuePath,
    bytes: &Vec<u8>,
) -> Result<(), io::Error> {
    let key = RegKey::predef(reg_value_path.hkey)
        .open_subkey_with_flags(reg_value_path.subkey_path, KEY_SET_VALUE)?;

    //TODO: See https://github.com/gentoo90/winreg-rs/issues/64 ("RegValue should contain Cow<[u8]>, not Vec<u8>").
    let unsafe_reg_value = ManuallyDrop::new(RegValue {
        vtype: RegType::REG_BINARY,
        // Unsafely double-owned `Vec`.
        bytes: unsafe { Vec::from_raw_parts(bytes.as_ptr() as _, bytes.len(), bytes.capacity()) },
    });

    // A panic would leak the reg value, but at least not cause a double-drop.
    let result = key.set_raw_value(reg_value_path.value_name, &unsafe_reg_value);

    // Drop only parts in fact owned. Use `ManuallyDrop` like `Vec::into_raw_parts()`, which is available in nightly Rust (as of Nov. 2023).
    let RegValue { bytes, .. } = ManuallyDrop::into_inner(unsafe_reg_value);
    let _ = ManuallyDrop::new(bytes);

    result?;
    Ok(())
}

pub(crate) fn export_reg_bin_values<T: AsRef<Path>>(
    reg_value_paths: &[RegValuePath],
    file_path: T,
) -> Result<(), io::Error> {
    //! # Panics
    //! Panics in case of an unknown `HKEY`. As of Nov. 2023, there are 10 that the `winreg` crate re-exports.

    let io_error = |_| io::Error::from(io::ErrorKind::Other);

    let mut text = String::with_capacity(2048);

    text.push_str("\u{feff}Windows Registry Editor Version 5.00\r\n");
    text.push_str("\r\n");

    for reg_value_path in reg_value_paths {
        write!(
            text,
            "[{}\\{}]\r\n",
            hkey_to_str(reg_value_path.hkey),
            reg_value_path.subkey_path
        )
        .map_err(io_error)?;

        write!(text, "\"{}\"=hex:", reg_value_path.value_name).map_err(io_error)?;

        let mut first = true;
        for byte in read_reg_bin_value(reg_value_path)? {
            write!(text, "{}{:02x}", if first { "" } else { "," }, byte).map_err(io_error)?;
            first = false;
        }

        text.push_str("\r\n");
        text.push_str("\r\n");
    }

    // Write file as UTF-16LE. BOM was added at the beginning. This is how `regedit.exe` saves .reg files.
    let mut file = File::create(file_path)?;
    for int16 in text.encode_utf16() {
        file.write(&int16.to_le_bytes())?;
    }

    Ok(())
}

pub(crate) fn delete_reg_value(reg_value_path: &RegValuePath) -> Result<(), io::Error> {
    let key = RegKey::predef(reg_value_path.hkey)
        .open_subkey_with_flags(reg_value_path.subkey_path, KEY_SET_VALUE)?;

    key.delete_value(reg_value_path.value_name)
        .or_else(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(error)
            }
        })
}

const fn hkey_to_str(hkey: HKEY) -> &'static str {
    use winreg::enums::*;

    match hkey {
        HKEY_CLASSES_ROOT => "HKEY_CLASSES_ROOT",
        HKEY_CURRENT_USER => "HKEY_CURRENT_USER",
        HKEY_LOCAL_MACHINE => "HKEY_LOCAL_MACHINE",
        HKEY_USERS => "HKEY_USERS",
        HKEY_PERFORMANCE_DATA => "HKEY_PERFORMANCE_DATA",
        HKEY_PERFORMANCE_TEXT => "HKEY_PERFORMANCE_TEXT",
        HKEY_PERFORMANCE_NLSTEXT => "HKEY_PERFORMANCE_NLSTEXT",
        HKEY_CURRENT_CONFIG => "HKEY_CURRENT_CONFIG",
        HKEY_DYN_DATA => "HKEY_DYN_DATA",
        HKEY_CURRENT_USER_LOCAL_SETTINGS => "HKEY_CURRENT_USER_LOCAL_SETTINGS",
        _ => panic!("unknown `HKEY`"),
    }
}
