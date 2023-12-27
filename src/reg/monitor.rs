use futures::{
    channel::oneshot,
    select,
    stream::{FusedStream, StreamExt},
    FutureExt,
};
use map_self::MapSelf;
use serde::Deserialize;
use std::{collections::HashMap, pin::Pin};
use thiserror::Error;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER,
        Security::{
            Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser,
            SID_AND_ATTRIBUTES, TOKEN_QUERY,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};
use windows_helpers::{dual_call, FirstCallExpectation, ResGuard};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_USERS};
use wmi::{query::quote_and_escape_wql_str, COMLibrary, WMIConnection, WMIError, WMIResult};

use super::{hkey_to_str, RegValuePath};

// Alternatively, a similar implementation could use `RegNotifyChangeKeyValue()`, which may be faster that WMI.
/// Note that, on changes in very quick succession, reading a registry value after receiving a change event for it may yield newer data than from the write that triggered the event.
pub struct RegValueMonitor<T: Copy> {
    _wmi_con: WMIConnection,
    ids_of_reg_value_changes: HashMap<RegValueChange, T>,
    event_stream: Pin<Box<dyn FusedStream<Item = WMIResult<RegValueChange>>>>,
}

impl<T: Copy> RegValueMonitor<T> {
    pub fn new<'a, I>(reg_value_paths: I) -> Result<Self, WMIError>
    where
        I: IntoIterator<Item = (T, &'a RegValuePath<'a>)>,
    {
        let wmi_con = WMIConnection::new(COMLibrary::new()?)?;

        let mut ids_of_reg_value_changes = HashMap::new();
        let mut sid = None;

        let mut query = String::from(r"SELECT * FROM RegistryValueChangeEvent WHERE");

        let mut first = true;
        for (id, reg_value_path) in reg_value_paths {
            //TODO: See <https://github.com/ohadravid/wmi-rs/issues/86> ("Helper to resolve registry links"). Otherwise, offer `current_user_sid()` to `whoami` crate.
            // Resolve links.
            let (corrected_hkey, subkey_path_prefix) = match reg_value_path.hkey {
                HKEY_CURRENT_USER => {
                    if sid.is_none() {
                        sid = Some(current_user_sid().map_err(|error| WMIError::HResultError {
                            hres: error.code().0,
                        })?);
                    }
                    (HKEY_USERS, sid.as_ref())
                }
                // (`HKEY_CLASSES_ROOT` links to `HKEY_LOCAL_MACHINE\SOFTWARE\Classes` as well as `HKEY_CURRENT_USER\SOFTWARE\Classes` in a merging way, which is why it can't be resolved here.)
                hkey => (hkey, None),
            };

            // Make proper path.
            let expected_reg_value_change = RegValueChange {
                hive: hkey_to_str(corrected_hkey).to_string(),
                key_path: if let Some(prefix) = subkey_path_prefix {
                    prefix.to_string() + r"\" + reg_value_path.subkey_path
                } else {
                    reg_value_path.subkey_path.to_string()
                },
                value_name: reg_value_path.value_name.to_string(),
            };

            // Build query.
            // (Parentheses aren't necessary: "When more than one logical operator is used in a statement, the OR operators are evaluated after the AND operators." [https://learn.microsoft.com/en-us/windows/win32/wmisdk/wql-sql-for-wmi])
            if !first {
                query.push_str(r" OR");
            }

            query.push_str(r" Hive=");
            query.push_str(&quote_and_escape_wql_str(&expected_reg_value_change.hive));

            query.push_str(r" AND KeyPath=");
            query.push_str(&quote_and_escape_wql_str(
                &expected_reg_value_change.key_path,
            ));

            query.push_str(r" AND ValueName=");
            query.push_str(&quote_and_escape_wql_str(
                &expected_reg_value_change.value_name,
            ));

            // Build `HashMap` to associate events with registry value IDs from user.
            ids_of_reg_value_changes.insert(expected_reg_value_change, id);

            first = false;
        }

        let event_stream = Box::pin(
            wmi_con
                .async_raw_notification::<RegValueChange>(query)?
                .fuse(),
        );

        Ok(Self {
            _wmi_con: wmi_con,
            ids_of_reg_value_changes,
            event_stream,
        })
    }

    pub async fn next_change(&mut self) -> Option<Result<T, WMIError>> {
        loop {
            break match self.event_stream.next().await {
                Some(result) => Some(match result {
                    Ok(changed_value) => {
                        Ok(match self.ids_of_reg_value_changes.get(&changed_value) {
                            Some(id) => *id,
                            // Skip unrelated nonsense, which shouldn't actually happen.
                            None => continue,
                        })
                    }
                    Err(error) => Err(error),
                }),
                None => None,
            };
        }
    }

    pub fn r#loop<F, U, E>(
        &mut self,
        stop_receiver: Option<oneshot::Receiver<U>>,
        mut callback: F,
    ) -> Result<U, MonitorLoopError<E>>
    where
        F: FnMut(T) -> Option<Result<U, E>>,
        U: Default,
    {
        //! Send a signal to the `stop_receiver` or return `Some(...)` from the callback to stop the loop.
        //!
        //! # Examples
        //! ```ignore
        //! fn main() {
        //!     let (stop_sender, stop_receiver) = oneshot::channel();
        //!
        //!     let join_handle = thread::spawn(move || {
        //!         let mut monitor = RegValueMonitor::new([
        //!             (NightLightRegValueId::State, &NIGHT_LIGHT_STATE_REG_VALUE_PATH),
        //!             (NightLightRegValueId::Settings, &NIGHT_LIGHT_SETTINGS_REG_VALUE_PATH),
        //!         ])
        //!         .unwrap();
        //!
        //!         monitor.r#loop(Some(stop_receiver), |changed_value_id| {
        //!             println!("{:?}", changed_value_id);
        //!             None
        //!         })
        //!         .unwrap();
        //!     });
        //!
        //!     thread::sleep(Duration::from_secs(10));
        //!     stop_sender.send(()).unwrap();
        //!     join_handle.join().unwrap();
        //! }
        //!
        //! #[derive(Clone, Copy, Debug)]
        //! enum NightLightRegValueId {
        //!     State,
        //!     Settings,
        //! }
        //! ```

        //. With no receiver, make one, so the loop works.
        let (_stop_sender, mut stop_receiver) = if let Some(orig_receiver) = stop_receiver {
            (None, orig_receiver)
        } else {
            oneshot::channel().map_self(|(sender, receiver)| (Some(sender), receiver))
        };

        futures::executor::block_on(async {
            loop {
                select! {
                    change_event = self.next_change().fuse() => {
                        match change_event {
                            // New change.
                            Some(Ok(id)) => if let Some(result) = callback(id) {
                                result.map_err(|err_value| MonitorLoopError::Other(err_value))?;
                            },
                            // Stream error.
                            Some(Err(error)) => break Err(MonitorLoopError::WmiError(error)),
                            // Stream should never be exhausted: "The `notification` method returns an iterator that waits for any incoming events resulting from the provided query. Loops reading from this iterator will not end until they are broken." (https://docs.rs/wmi/latest/wmi/#subscribing-to-event-notifications)
                            None => unreachable!(),
                        }
                    },
                    // User desires to stop loop.
                    value = stop_receiver => break Ok(value.unwrap_or_default()),
                }
            }
        })
    }
}

#[derive(Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename = "RegistryValueChangeEvent")]
#[serde(rename_all = "PascalCase")]
struct RegValueChange {
    hive: String,
    key_path: String,
    value_name: String,
}

#[derive(Error, Debug)]
pub enum MonitorLoopError<T> {
    #[error("WMI error: {0}")]
    WmiError(#[from] WMIError),
    #[error("monitor loop error: {0}")]
    Other(T),
}

fn current_user_sid() -> Result<String, windows::core::Error> {
    let process_token_handle = ResGuard::with_mut_acq_and_close_handle(|handle| unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, handle)
    })?;

    let mut sid_and_attrs_buffer = Vec::<u8>::new();
    let mut sid_and_attrs_buffer_size = 0;

    dual_call(
        FirstCallExpectation::Win32Error(ERROR_INSUFFICIENT_BUFFER),
        |getting_buffer_size| unsafe {
            GetTokenInformation(
                *process_token_handle,
                TokenUser,
                (!getting_buffer_size).then(|| {
                    sid_and_attrs_buffer.resize(sid_and_attrs_buffer_size as _, 0);
                    sid_and_attrs_buffer.as_mut_ptr().cast()
                }),
                sid_and_attrs_buffer_size,
                &mut sid_and_attrs_buffer_size,
            )
        },
    )?;

    let string_sid = unsafe {
        ResGuard::<PWSTR>::with_mut_acq_and_local_free(|pwstr| {
            ConvertSidToStringSidW(
                (&*sid_and_attrs_buffer.as_ptr().cast::<SID_AND_ATTRIBUTES>()).Sid,
                pwstr,
            )
        })?
        .to_string()?
    };

    Ok(string_sid)
}
