//! Types to retrieve information about and control the Windows Night Light feature.
//!
//! In regular use cases, you should use the `NightLight` type. With the underlying raw types, which are also public, there can be unexpected behavior or inconsistent states that, when written to the registry, may not get resolved until restart of the OS or at least logging off:
//!
//! - Changing schedule-related settings - written to the settings registry value - may entail the state registry value - incl. the active-state - being changed by the OS. Trying to change both the settings and state registry value in close temporal proximity in an irreconcilable way may lead to one not being applied.
//! - Night preview must not be switched or held active while changing anything that may potentially switch the active-state (incl. schedule-related settings).
//! - After logging on to the OS, changing the color temperature on its own remains ineffective if preview mode wasn't at least once activated in the session. (Briefly activating is also restores the color temperature after turning the screen back on.) If Night Light is off, changing the temperature and then, not in very close temporal proximity, turning Night Light on, correctly applies the temperature in that instance, however.
//!
//! With all types, there can be race conditions when simultaneously changing the Night Light settings elsewhere in the system. This is why you should read, mutate and write without delays in between. `NightLight` instances expire after a short duration to enforce this.
//!
//! The `NightLight` type encapsulates both the state and settings registry value and only writes one when you de facto changed its properties, compared with the data retrieved on instance creation, failing if changes don't harmonize with other properties (changed or unchanged). Using `NightLight` twice in direct succession won't help you writing both registry values in an irreconcilable way (the error may just be silent). If you need to do that, use a delay between writing a `NightLight` instance to registry and creating the next, causing the state registry value with the active-state to be changed last.

mod settings;
mod state;
mod time;

use chrono::SecondsFormat;
use convert_case::{Case, Casing};
use core::fmt;
use futures::channel::oneshot;
use serde_json::json;
pub use settings::{RawNightLightSettings, ScheduleType};
pub use state::{RawNightLightState, TransitionCause};
use std::{
    io,
    ops::Sub,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime},
};
pub use time::{ClockTime, ClockTimeFrame, Meridiem};
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE},
    RegKey,
};

use crate::{
    data_conversion::{
        format::write_table,
        time::{
            epoch_duration_to_filetime, utc_epoch_secs_to_local_iso_string,
            utc_filetime_to_local_date_time, utc_filetime_to_local_iso_string,
        },
        ParseError, Strictness,
    },
    reg::{
        delete_reg_value, export_reg_bin_values,
        monitor::{MonitorLoopError, RegValueMonitor},
        read_reg_bin_value, write_reg_bin_value, RegValuePath,
    },
};

pub struct NightLight {
    state: RawNightLightState,
    settings: RawNightLightSettings,
    sunset_to_sunrise_possible: Option<bool>,
    uses_12_hour_clock: bool,
    loaded_instant: Instant,
    strictness: Strictness,
}

impl NightLight {
    /// The lowest Kelvin value last known to be valid (as of Oct. 2023).
    pub const MIN_NIGHT_COLOR_TEMP: u16 = 1200;
    pub const MAX_NIGHT_COLOR_TEMP: u16 = 6500;

    /// Alias for minimum.
    pub const WARMEST_NIGHT_COLOR_TEMP: u16 = Self::MIN_NIGHT_COLOR_TEMP;
    /// Alias for maximum.
    pub const COLDEST_NIGHT_COLOR_TEMP: u16 = Self::MAX_NIGHT_COLOR_TEMP;

    /// Official default as of Dec. 2023. Found out by alternatingly setting the warmth factor to `None` and a concrete value until no difference could be visually perceived anymore, then snapping to a plausible round number.
    pub const DEFAULT_NIGHT_COLOR_TEMP: u16 = 4000;
    /// Equivalent of [`Self::DEFAULT_NIGHT_COLOR_TEMP`]. See [`Self::warmth()`].
    pub const DEFAULT_WARMTH: f32 = 1.0
        - (Self::DEFAULT_NIGHT_COLOR_TEMP - Self::MIN_NIGHT_COLOR_TEMP) as f32
            / (Self::MAX_NIGHT_COLOR_TEMP - Self::MIN_NIGHT_COLOR_TEMP) as f32;

    /// The delay [`Self::init_with_strictness()`] should wait for, if no better information is available. (Defined as a common animation duration.)
    pub const REASONABLE_INIT_DELAY: Duration = Duration::from_millis(200);

    /// Duration after which an instance expires. May be shortened in future versions.
    pub const EXPIRATION_TIMEOUT: Duration = Duration::from_millis(1000);

    pub fn from_reg() -> Result<Self, self::Error> {
        //! Creates a strict instance using [`Self::from_reg_with_strictness()`].

        Self::from_reg_with_strictness(Strictness::Strict)
    }

    pub fn from_reg_lenient() -> Result<Self, self::Error> {
        Self::from_reg_with_strictness(Strictness::Lenient)
    }

    pub fn from_reg_with_strictness(strictness: Strictness) -> Result<Self, self::Error> {
        //! Returns a fallback instance if one of the registry values doesn't exist in lenient mode.

        Ok(match NightLightBytes::from_reg() {
            Ok(bytes) => Self::from_bytes_with_strictness(bytes, strictness)?,
            Err(error) => {
                if error.kind() == io::ErrorKind::NotFound && strictness.is_lenient() {
                    Self::lenient_fallback()
                } else {
                    Err(error)?
                }
            }
        })
    }

    pub fn from_bytes(bytes: NightLightBytes) -> Result<Self, ParseError> {
        Self::from_bytes_with_strictness(bytes, Strictness::Strict)
    }

    pub fn from_bytes_lenient(bytes: NightLightBytes) -> Result<Self, ParseError> {
        Self::from_bytes_with_strictness(bytes, Strictness::Lenient)
    }

    pub fn from_bytes_with_strictness(
        bytes: NightLightBytes,
        strictness: Strictness,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            state: RawNightLightState::from_bytes(bytes.state, strictness)?,
            settings: RawNightLightSettings::from_bytes(bytes.settings, strictness)?,
            sunset_to_sunrise_possible: Self::sunset_to_sunrise_possible(),
            uses_12_hour_clock: false,
            loaded_instant: Instant::now(),
            strictness,
        })
    }

    pub fn lenient_fallback() -> Self {
        let now = SystemTime::now();
        Self {
            state: RawNightLightState::lenient_fallback(now),
            settings: RawNightLightSettings::lenient_fallback(now),
            sunset_to_sunrise_possible: Self::sunset_to_sunrise_possible(),
            uses_12_hour_clock: false,
            loaded_instant: Instant::now(),
            strictness: Strictness::Lenient,
        }
    }

    pub fn init(delay: Duration, also_wait_after: bool) -> Result<(), self::Error> {
        //! Initializes with strict instances using [`Self::init_with_strictness()`].

        Self::init_with_strictness(delay, also_wait_after, Strictness::Strict)
    }

    pub fn init_with_strictness(
        delay: Duration,
        also_wait_after: bool,
        strictness: Strictness,
    ) -> Result<(), self::Error> {
        //! Performs actions that are necessary after OS log-on to be able to change the color temperature without activating one of the boolean states. Also restores a warm color temperature after turning the screen back on. (As of Dec. 2023.)
        //!
        //! Concretely, it activates preview mode, waits for the delay duration, and deactivates it again. Doesn't write a registry value and wait, if preview mode was already active. To make the intermediate previewing as invisible as possible, the color temperature is temporarily set to the coldest possible, if Night Light is inactive.
        //!
        //! Normally to be used with [`Self::REASONABLE_INIT_DELAY`]. Additionally waiting after the last write can be activated in a command line context, if the user plans to follow the action up with another write.

        let mut inst = Self::from_reg_with_strictness(strictness)?;
        if !inst.night_preview_active() {
            // Activate preview mode.
            let previous_temp = if inst.active() {
                None
            } else {
                let temp = inst.night_color_temp();
                inst.set_night_color_temp(Some(Self::COLDEST_NIGHT_COLOR_TEMP)); // Make it invisible.
                temp
            };
            inst.set_night_preview_active(true);
            inst.write_to_reg()?;

            thread::sleep(delay);

            // Deactivate preview mode.
            let mut inst = Self::from_reg_with_strictness(strictness)?;
            if let Some(temp) = previous_temp {
                inst.set_night_color_temp(Some(temp));
            };
            inst.set_night_preview_active(false);
            inst.write_to_reg()?;

            if also_wait_after {
                thread::sleep(delay);
            }
        }

        Ok(())
    }

    pub fn export_reg<T: AsRef<Path>>(file_path: T) -> Result<(), io::Error> {
        //! Writes the Night Light registry values to a file in .reg file format.

        export_reg_bin_values(
            &[
                RawNightLightState::REG_VALUE_PATH,
                RawNightLightSettings::REG_VALUE_PATH,
            ],
            file_path,
        )
    }

    pub fn delete_reg() -> Result<(), io::Error> {
        //! Deletes the Night Light registry values to reset the Windows feature. May help when they've been corrupted and Night Light became unusable. User should restart or at least log-off after deletion.

        // Deletion order may be relevant. This order made the fewest problems so far.
        delete_reg_value(&RawNightLightSettings::REG_VALUE_PATH)?;
        delete_reg_value(&RawNightLightState::REG_VALUE_PATH)?;

        Ok(())
    }

    pub fn monitor<F, T, E>(
        stop_receiver: Option<oneshot::Receiver<T>>,
        mut callback: F,
    ) -> Result<T, MonitorLoopError<E>>
    where
        F: FnMut(RegValueId) -> Option<Result<T, E>>,
        T: Default,
    {
        let mut monitor = RegValueMonitor::new([
            (RegValueId::State, &RawNightLightState::REG_VALUE_PATH),
            (RegValueId::Settings, &RawNightLightSettings::REG_VALUE_PATH),
        ])?;

        monitor.r#loop(stop_receiver, |value_id| callback(value_id))
    }

    pub fn sunset_to_sunrise_possible() -> Option<bool> {
        //! Whether the "Sunset to sunrise" option is available, because location services are turned on. If not, the explicit schedule is the fallback. Returns `None` on registry access failure.

        const SUBKEY_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location";
        let reg_value_paths = [
            // Local machine.
            RegValuePath {
                hkey: HKEY_LOCAL_MACHINE,
                subkey_path: SUBKEY_PATH,
                value_name: "Value",
            },
            // Current user apps.
            RegValuePath {
                hkey: HKEY_CURRENT_USER,
                subkey_path: SUBKEY_PATH,
                value_name: "Value",
            },
            // Current user desktop apps.
            RegValuePath {
                hkey: HKEY_CURRENT_USER,
                subkey_path: &format!(r"{SUBKEY_PATH}\NonPackaged"),
                value_name: "Value",
            },
        ];

        for RegValuePath {
            hkey,
            subkey_path,
            value_name,
        } in reg_value_paths
        {
            if RegKey::predef(hkey)
                .open_subkey_with_flags(subkey_path, KEY_QUERY_VALUE)
                .ok()?
                .get_value::<String, _>(value_name)
                .ok()?
                != "Allow"
            {
                return Some(false);
            }
        }

        Some(true)
    }

    pub fn active(&self) -> bool {
        //! Whether night time color temperature is currently in effect, be it because manually chosen or by schedule.

        *self.state.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.state.active.set(active);
    }

    pub fn transition_cause(&self) -> TransitionCause {
        self.state.transition_cause
    }

    pub fn state_modified_filetime(&self) -> i64 {
        //! The state registry value's modification timestamp as a [`FILETIME`](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime) (always greater than 0).

        self.state.modified_filetime
    }

    pub fn latest_possible_settings_modified_epoch_secs(&self) -> u32 {
        //! The settings registry value's modification timestamp, or a value somewhat later when many changes were performed in quick succession (e.g., by moving the slider in the official settings).

        self.settings.prologue_epoch_secs
    }

    pub fn schedule_active(&self) -> bool {
        *self.settings.schedule_active
    }

    pub fn set_schedule_active(&mut self, schedule_active: bool) {
        self.settings.schedule_active.set(schedule_active);
    }

    pub fn schedule_type(&self) -> ScheduleType {
        //! The theoretical schedule type. The one really in effect also depends on the state of location services. See also other getter.

        *self.settings.schedule_type
    }

    pub fn effective_schedule_type(&self) -> Option<ScheduleType> {
        //! The schedule type really in effect, which also depends on location services being turned on or off. It's unknown how Windows behaves when location services are turned on, but the location still can't be retrieved for a longer period of time. Returns `None` on schedule type "Sunset to sunrise", if [`Self::sunset_to_sunrise_possible()`] did.

        match *self.settings.schedule_type {
            ScheduleType::SunsetToSunrise => {
                if self.sunset_to_sunrise_possible? {
                    Some(ScheduleType::SunsetToSunrise)
                } else {
                    Some(ScheduleType::Explicit)
                }
            }
            ScheduleType::Explicit => Some(ScheduleType::Explicit),
        }
    }

    pub fn set_schedule_type(&mut self, schedule_type: ScheduleType) {
        //! Because the explicit schedule is the fallback of "Sunset to sunrise", there won't be a change of the effective schedule type in the official Night Light settings when location services are off. The change is still perfomed however - just that it can only become effective when location services are turned on.

        self.settings.schedule_type.set(schedule_type);
    }

    pub fn sunset_to_sunrise(&self) -> Option<ClockTimeFrame> {
        //! Returns `None`, if all of the respective hour and minute values in the registry value were zero.

        self.settings.sunset_to_sunrise
    }

    pub fn scheduled_night(&self) -> ClockTimeFrame {
        //! The clock times defining the explicit schedule.

        *self.settings.scheduled_night
    }

    pub fn set_scheduled_night(&mut self, scheduled_night: ClockTimeFrame) {
        //! You can use *any* combination of clock times precise to the minute, and it will be adhered to. The time pickers in the official Night Light settings, however, will just display the times with 15-minute accuracy, rounded down. Two times the same clock time means zero-length night.

        self.settings.scheduled_night.set(scheduled_night);
    }

    pub fn night_color_temp(&self) -> Option<u16> {
        //! The night time color temperature in Kelvin. May possibly be out of the range of the constants, if Microsoft changed them. Returns `None`, if the information wasn't present in the registry value, in which case Windows applies the default.

        *self.settings.night_color_temp
    }

    pub fn night_color_temp_in_range(&self) -> Option<u16> {
        //! The color temperature guaranteed to be in the range of the constants.

        self.settings
            .night_color_temp
            .map(|temp| temp.clamp(Self::MIN_NIGHT_COLOR_TEMP, Self::MAX_NIGHT_COLOR_TEMP))
    }

    pub fn set_night_color_temp(&mut self, night_color_temp: Option<u16>) {
        //! Windows corrects the value to lie in the valid range. `None` makes Windows apply the default.

        self.settings.night_color_temp.set(night_color_temp);
    }

    pub fn warmth(&self) -> Option<f32> {
        //! A factor in the range from 0 to 1, based on the color temperature range constants. May return `None` like the color temperature getter. Corresponds to the "Strength" slider in the official Night Light settings, which shows a percentage.

        self.night_color_temp_in_range().map(|temp| {
            1.0 - (temp - Self::MIN_NIGHT_COLOR_TEMP) as f32
                / (Self::MAX_NIGHT_COLOR_TEMP - Self::MIN_NIGHT_COLOR_TEMP) as f32
        })
    }

    pub fn set_warmth(&mut self, warmth: Option<f32>) {
        //! Steps in the upper range are perceived as more intense, which is why they should be smaller to achieve the same step in perception as larger steps in the lower range. There isn't a one-size-fits-all correction curve, since color temperature perception also depends on light intensity (see <https://en.wikipedia.org/wiki/Kruithof_curve>). But gamma correction (exponentiation) still seems suitable.
        //!
        //! # Panics
        //! Panics on NaN.

        self.set_night_color_temp(warmth.map(|warmth| {
            if warmth.is_nan() {
                panic!("value is NaN");
            }

            let precise_temp = (Self::MAX_NIGHT_COLOR_TEMP - Self::MIN_NIGHT_COLOR_TEMP) as f32
                * (1.0 - warmth)
                + Self::MIN_NIGHT_COLOR_TEMP as f32;
            precise_temp.round().clamp(0f32, u16::MAX as f32) as u16
        }));
    }

    pub fn night_preview_active(&self) -> bool {
        //! Whether preview mode with a hard change (as opposed to a smooth transition) to night color temperature is in effect. The official Night Light settings activate this while moving the color temperature slider.

        *self.settings.night_preview_active
    }

    pub fn set_night_preview_active(&mut self, night_preview_active: bool) {
        self.settings.night_preview_active.set(night_preview_active);
    }

    pub fn set_uses_12_hour_clock(&mut self, uses_12_hour_clock: bool) {
        //! Only for display purposes.

        self.uses_12_hour_clock = uses_12_hour_clock;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&json!({
            "active": *self.state.active,
            "transitionCause": format!("{:?}", self.state.transition_cause).to_case(Case::Camel),
            "stateModifiedTimestamp": utc_filetime_to_local_iso_string(self.state.modified_filetime).expect("`FILETIME` should be valid"),

            "latestPossibleSettingsModifiedTimestamp": utc_epoch_secs_to_local_iso_string(self.settings.prologue_epoch_secs).expect("epoch secs should be valid"),
            "scheduleActive": *self.settings.schedule_active,
            "scheduleType": format!("{:?}", *self.settings.schedule_type).to_case(Case::Camel),
            "sunsetToSunrisePossible": self.sunset_to_sunrise_possible,
            "effectiveScheduleType": self.effective_schedule_type().map(|r#type| format!("{:?}", r#type).to_case(Case::Camel)),
            "sunsetToSunrise": self.settings.sunset_to_sunrise,
            "scheduledNight": *self.settings.scheduled_night,
            "nightColorTemp": *self.settings.night_color_temp,
            "warmth": self.warmth(),
            "nightPreviewActive": *self.settings.night_preview_active,
        }))
        .expect("serializing to JSON shouldn't fail")
    }

    pub fn write_to_reg(mut self) -> Result<(), self::Error> {
        //! Writes the data to the registry values, which immediately applies it.

        if self.loaded_instant.elapsed() > Self::EXPIRATION_TIMEOUT {
            return Err(DataError::Expired.into());
        }

        let (state_changed, settings_changed) = self.verify_state_and_settings()?;

        let state_bytes = state_changed.then(|| {
            //. Only Windows is allowed to write the other value, because it does so by schedule.
            self.state.transition_cause = TransitionCause::Manual;

            self.state.to_bytes()
        });
        let settings_bytes = settings_changed.then(|| self.settings.to_bytes());

        // Write settings first, then state.
        if let Some(settings_bytes) = settings_bytes {
            write_reg_bin_value(&RawNightLightSettings::REG_VALUE_PATH, &settings_bytes)?;
            // (When state-changing settings were changed, Windows may now change the state registry value.)
        }
        if let Some(state_bytes) = state_bytes {
            write_reg_bin_value(&RawNightLightState::REG_VALUE_PATH, &state_bytes)?;
        }

        Ok(())
    }

    fn verify_state_and_settings(&mut self) -> Result<(bool, bool), DataError> {
        //! Returns whether the state and the settings were changed.

        let active_changed = self.state.active.changed();

        let schedule_active_changed = self.settings.schedule_active.changed();
        let schedule_type_changed = self.settings.schedule_type.changed();
        let scheduled_night_changed = self.settings.scheduled_night.changed();
        let night_color_temp_changed = self.settings.night_color_temp.changed();
        let night_preview_active_changed = self.settings.night_preview_active.changed();

        let state_changed = active_changed;
        let settings_changed = schedule_active_changed
            || schedule_type_changed
            || scheduled_night_changed
            || night_color_temp_changed
            || night_preview_active_changed;
        let state_changing_settings_changed =
            schedule_active_changed || schedule_type_changed || scheduled_night_changed;

        if state_changed && state_changing_settings_changed {
            return Err(DataError::Irreconcilable(
                CompetingProps::StateVsStateChangingSettings,
            ));
        }

        if (*self.settings.night_preview_active /*turned on or held active*/ || night_preview_active_changed/*turned off*/)
            && (state_changed || state_changing_settings_changed)
        {
            return Err(if night_preview_active_changed {
                DataError::Irreconcilable(if state_changed {
                    CompetingProps::StateVsNightPreview
                } else {
                    CompetingProps::StateChangingSettingsVsNightPreview
                })
            } else {
                // Unchanged, but active night preview. Interfering with other software in this state makes adverse effects likely.
                DataError::NightPreviewInProgress
            });
        }

        Ok((state_changed, settings_changed))
    }
}

impl fmt::Display for NightLight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bool_to_yes_no = |flag| if flag { "yes" } else { "no" }.to_string();
        let parenthesize_if = |flag, string| if flag { format!("({string})") } else { string };

        let effective_schedule_type = self.effective_schedule_type();

        write_table(
            f,
            &[
                Some(("Active", bool_to_yes_no(*self.state.active))),
                Some((
                    "Transition Cause",
                    format!("{:?}", self.state.transition_cause).to_case(Case::Lower),
                )),
                None,
                Some((
                    "Warmth",
                    self.warmth()
                        .map(|warmth| format!("{warmth:.2}"))
                        .unwrap_or_else(|| format!("default (should be {})", Self::DEFAULT_WARMTH)),
                )),
                Some((
                    "Kelvin",
                    self.settings
                        .night_color_temp
                        .map(|temp| temp.to_string())
                        .unwrap_or_else(|| {
                            format!("default (should be {})", Self::DEFAULT_NIGHT_COLOR_TEMP)
                        }),
                )),
                Some((
                    "Preview Active",
                    bool_to_yes_no(*self.settings.night_preview_active),
                )),
                None,
                Some((
                    "Schedule Active",
                    bool_to_yes_no(*self.settings.schedule_active),
                )),
                Some((
                    "Schedule Type (Effective)",
                    effective_schedule_type
                        .map(|r#type| format!("{:?}", r#type).to_case(Case::Lower))
                        .unwrap_or_else(|| "N/A".to_string()),
                )),
                Some((
                    "Sunset to Sunrise",
                    parenthesize_if(
                        effective_schedule_type == Some(ScheduleType::Explicit),
                        self.settings
                            .sunset_to_sunrise
                            .map(|frame| frame.format(self.uses_12_hour_clock))
                            .unwrap_or_else(|| "N/A".to_string()),
                    ),
                )),
                Some((
                    "Explicit Night",
                    parenthesize_if(
                        effective_schedule_type == Some(ScheduleType::SunsetToSunrise),
                        self.settings
                            .scheduled_night
                            .format(self.uses_12_hour_clock),
                    ),
                )),
                None,
                Some((
                    "Modified (Latest Possible)",
                    utc_filetime_to_local_date_time(self.state.modified_filetime.max(
                        epoch_duration_to_filetime(Duration::from_secs(
                            self.settings.prologue_epoch_secs as _,
                        )),
                    ))
                    .ok_or(fmt::Error)?
                    .format(if self.uses_12_hour_clock {
                        "%Y-%m-%d, %I:%M:%S %P"
                    } else {
                        "%Y-%m-%d, %H:%M:%S"
                    })
                    .to_string(),
                )),
            ],
        )?;

        Ok(())
    }
}

impl fmt::Debug for NightLight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_table(
            f,
            &[
                Some((
                    "prologue timestamp (state)",
                    utc_epoch_secs_to_local_iso_string(self.state.prologue_epoch_secs)
                        .ok_or(fmt::Error)?,
                )),
                Some(("active (state)", self.state.active.to_string())),
                Some((
                    "transition cause (state)",
                    format!("{:?}", self.state.transition_cause),
                )),
                Some((
                    "modified-`FILETIME` (state)",
                    utc_filetime_to_local_iso_string(self.state.modified_filetime)
                        .ok_or(fmt::Error)?,
                )),
                None,
                Some((
                    "prologue timestamp (settings)",
                    utc_epoch_secs_to_local_iso_string(self.settings.prologue_epoch_secs)
                        .ok_or(fmt::Error)?,
                )),
                Some((
                    "schedule active (settings)",
                    self.settings.schedule_active.to_string(),
                )),
                Some((
                    "schedule type (settings)",
                    format!("{:?}", *self.settings.schedule_type),
                )),
                Some((
                    "sunset-to-sunrise possible (other)",
                    format!("{:?}", self.sunset_to_sunrise_possible),
                )),
                Some((
                    "effective schedule type (settings & other)",
                    format!("{:?}", self.effective_schedule_type()),
                )),
                Some((
                    "sunset to sunrise (settings)",
                    self.settings
                        .sunset_to_sunrise
                        .map(|frame| frame.format(self.uses_12_hour_clock))
                        .unwrap_or_else(|| format!("{:?}", None::<()>)),
                )),
                Some((
                    "scheduled night (settings)",
                    self.settings
                        .scheduled_night
                        .format(self.uses_12_hour_clock),
                )),
                Some((
                    "night color temp. (settings)",
                    format!("{:?}", *self.settings.night_color_temp),
                )),
                Some((
                    "warmth (settings, processed)",
                    format!("{:?}", self.warmth()),
                )),
                Some((
                    "night preview active (settings)",
                    self.settings.night_preview_active.to_string(),
                )),
                None,
                Some((
                    "loaded-`Instant`",
                    chrono::Local::now()
                        .sub(self.loaded_instant.elapsed())
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                )),
                Some(("strictness", format!("{:?}", self.strictness))),
            ],
        )?;

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error interacting with the registry, e.g., because of non-existent registry value. If the user never used Night Light in their OS installation, they should be advised to change something in the official settings to create the registry values and try again (turning Night Light on/off and moving the temperature slider should suffice).
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    /// Couldn't parse a byte stream.
    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),
    /// Couldn't serialize the data from an instance into a byte stream.
    #[error("data error: {0}")]
    DataError(#[from] DataError),
}

#[derive(thiserror::Error, Debug)]
pub enum DataError {
    /// The object expired to enforce avoidance of race conditions.
    #[error("object expired: duration between reading and writing was too long")]
    Expired,
    /// Changed properties don't harmonize with the condition of other properties.
    #[error("changed props are irreconcilable with other props: {0}")]
    Irreconcilable(CompetingProps),
    /// Night preview is currently active, because of other software or earlier use of this crate. The user could, e.g., right now be dragging the color tempature slider in the official Night Light settings.
    #[error("night preview was active while trying to change props irreconcilable with it")]
    NightPreviewInProgress,
}

#[derive(Clone, Debug)]
pub struct NightLightBytes {
    pub state: Vec<u8>,
    pub settings: Vec<u8>,
}

impl NightLightBytes {
    pub fn from_reg() -> Result<Self, io::Error> {
        Ok(Self {
            state: read_reg_bin_value(&RawNightLightState::REG_VALUE_PATH)?,
            settings: read_reg_bin_value(&RawNightLightSettings::REG_VALUE_PATH)?,
        })
    }

    pub fn bytes_of_value(&self, reg_value_id: RegValueId) -> &[u8] {
        match reg_value_id {
            RegValueId::State => &*self.state,
            RegValueId::Settings => &*self.settings,
        }
    }
}

#[derive(Debug)]
pub enum CompetingProps {
    StateVsStateChangingSettings,
    StateVsNightPreview,
    StateChangingSettingsVsNightPreview,
}

impl fmt::Display for CompetingProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompetingProps::StateVsStateChangingSettings => {
                write!(f, "state vs. state-changing settings")
            }
            CompetingProps::StateVsNightPreview => write!(f, "state vs. night preview"),
            CompetingProps::StateChangingSettingsVsNightPreview => {
                write!(f, "state-changing settings vs. night preview")
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RegValueId {
    State,
    Settings,
}

#[cfg(test)]
mod tests {
    use crate::cloud_store::night_light::NightLight;

    #[ignore]
    #[test]
    fn playground() -> Result<(), super::Error> {
        let mut night_light = NightLight::from_reg()?;

        // night_light.set_active(true);

        night_light.set_schedule_active(true);
        // night_light.set_schedule_type(ScheduleType::Explicit);
        // night_light.set_scheduled_night(ClockTimeFrame {
        //     start: ClockTime::from_h_min(23, 6)?,
        //     end: ClockTime::from_h_min(6, 42)?,
        // });

        // night_light.set_night_color_temp(2000);
        // night_light.set_warmth(0.7);

        // night_light.set_night_preview_active(true);

        night_light.write_to_reg()?;

        Ok(())
    }

    #[test]
    fn from_reg_strict() {
        let result = NightLight::from_reg();
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn from_reg_lenient() {
        let result = NightLight::from_reg_lenient();
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn compare_strict_with_lenient_reg_result() {
        let strict_result = NightLight::from_reg();
        let lenient_result = NightLight::from_reg_lenient();

        if let (Ok(strict), Ok(lenient)) = (strict_result, lenient_result) {
            assert!(strict.state == lenient.state, "unequal states");
            assert!(strict.settings == lenient.settings, "unequal settings");
        } else {
            panic!("no `NightLight`");
        }
    }

    #[test]
    fn sunset_to_sunrise_possible_is_some() {
        assert!(NightLight::sunset_to_sunrise_possible().is_some());
    }

    #[test]
    fn verify_warmth_setter() -> Result<(), super::Error> {
        let mut night_light = NightLight::from_reg()?;

        assert_eq!(NightLight::WARMEST_NIGHT_COLOR_TEMP, 1200);
        assert_eq!(NightLight::COLDEST_NIGHT_COLOR_TEMP, 6500);

        night_light.set_warmth(Some(0.7));
        assert_eq!(night_light.night_color_temp(), Some(2790));

        Ok(())
    }
}
