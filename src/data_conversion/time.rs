use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local, SecondsFormat};

const HECTONANOS_1601_TO_1970: i64 = 11644473600_000_000_0;
const HECTONANOS_PER_SEC: i64 = 1_000_000_0;

/// See <https://stackoverflow.com/questions/9999393/latest-possible-filetime/18188484#18188484>.
pub const LATEST_FILETIME: i64 = 0x7fff35f4f06c58f0;

pub fn system_time_to_epoch_duration(system_time: SystemTime) -> Duration {
    //! Ceils the result to 0, making 1970 the earliest time.

    system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}

pub fn now_as_epoch_duration() -> Duration {
    system_time_to_epoch_duration(SystemTime::now())
}

pub fn epoch_duration_to_epoch_secs(duration: Duration) -> u32 {
    //! # Panics
    //! Panics if the time is in the distant past or future.

    duration
        .as_secs()
        .try_into()
        .expect("now shouldn't be distant past or future")
}

pub fn epoch_duration_to_filetime(duration: Duration) -> i64 {
    (duration.as_nanos() / 100) as i64 + HECTONANOS_1601_TO_1970
}

pub fn utc_epoch_secs_to_local_iso_string(secs: u32) -> Option<String> {
    Some(
        DateTime::from_timestamp(secs as _, 0)?
            .with_timezone(&Local)
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}

pub fn utc_filetime_to_local_iso_string(filetime: i64) -> Option<String> {
    let date_time = utc_filetime_to_local_date_time(filetime)?;

    let mut string = date_time.to_rfc3339_opts(SecondsFormat::Nanos, true);
    string.truncate(27);
    string.push_str(&date_time.format("%:z").to_string());

    Some(string)
}

pub fn utc_filetime_to_local_date_time(filetime: i64) -> Option<DateTime<Local>> {
    Some(
        DateTime::from_timestamp(
            (filetime - HECTONANOS_1601_TO_1970) / HECTONANOS_PER_SEC,
            (filetime % HECTONANOS_PER_SEC * 100) as _,
        )?
        .with_timezone(&Local),
    )
}
