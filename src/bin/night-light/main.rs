mod cli;

use anyhow::anyhow;
use clap::Parser;
use colored::Colorize;
use futures::channel::oneshot;
use std::{
    iter,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, LPARAM, LRESULT, WPARAM},
        System::{
            Console::{FreeConsole, GetConsoleProcessList},
            Power::RegisterPowerSettingNotification,
            SystemServices::{PowerMonitorOn, GUID_CONSOLE_DISPLAY_STATE, MONITOR_DISPLAY_STATE},
            Threading::GetCurrentProcessId,
        },
        UI::WindowsAndMessaging::{
            DestroyWindow, PostMessageW, PostQuitMessage, RegisterWindowMessageW,
            SendNotifyMessageW, DEVICE_NOTIFY_WINDOW_HANDLE, HWND_BROADCAST, WM_CREATE, WM_DESTROY,
            WM_POWERBROADCAST,
        },
    },
};
use windows_helpers::{
    core::CheckNumberError,
    dual_call,
    power::PowerBroadcastSettingExt,
    win32_app::{
        error::{try_or_quit_now, try_then_favor_app_error},
        msg_loop,
        window::{translate_power_broadcast_msg, PowerBroadcastMsg, Window, WindowClass},
    },
    FirstCallExpectation, ResGuard,
};

use cli::{Cli, InitDurationArg, RequiredOnOffArgs, ScheduleArgs, Subcmd, TempArgs};
use sem_reg::{
    cloud_store::night_light::{self, NightLight, NightLightBytes},
    data_conversion::{hex_bytes::HexBytes, Strictness},
};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcmd {
        // Export so that the user can be supported, e.g.
        Some(Subcmd::Export { output }) => {
            let has_user_defined_path = output.is_some();
            let file_path = output.unwrap_or_else(|| {
                chrono::Local::now()
                    .format(if cli.am_pm {
                        "%Y-%m-%d, %I.%M.%S %P.reg"
                    } else {
                        "%Y-%m-%d, %H.%M.%S.reg"
                    })
                    .to_string()
            });

            NightLight::export_reg(&file_path)?;

            if !has_user_defined_path {
                println!("Wrote '{file_path}'.");
            }
        }

        Some(Subcmd::Delete) => {
            NightLight::delete_reg()?;
        }

        Some(Subcmd::Monitor) => {
            println!("Press Ctrl+C to abort. (On very fast changes, newer data than that triggering the change may be read.)");
            println!();

            let (stop_sender, stop_receiver) = oneshot::channel::<()>();
            let mut stop_sender = Some(stop_sender);
            ctrlc::set_handler(move || {
                if let Some(stop_sender) = stop_sender.take() {
                    stop_sender.send(()).unwrap();
                }
            })?;

            let mut previous_bytes = NightLightBytes::from_reg()?;

            NightLight::monitor(Some(stop_receiver), |value_id| {
                let bytes = match NightLightBytes::from_reg() {
                    Ok(bytes) => bytes,
                    Err(error) => return Some(Err(night_light::Error::from(error))),
                };

                println!(
                    "{}",
                    format!("{value_id:?} registry value changed").to_uppercase()
                );

                //. When parsing fails, the user must at least see the bytes to be able to ask for support.
                let hex_bytes = HexBytes::new(bytes.bytes_of_value(value_id));
                println!("{}", format!("(bytes: {})", hex_bytes).dimmed());

                println!(
                    "(diff against previous: {})",
                    hex_bytes.diff_against(previous_bytes.bytes_of_value(value_id))
                );
                println!();

                previous_bytes = bytes.clone();

                let mut night_light = match NightLight::from_bytes(bytes) {
                    Ok(night_light) => night_light,
                    Err(error) => return Some(Err(error.into())),
                };
                night_light.set_uses_12_hour_clock(cli.am_pm);
                println!("{night_light:?}");
                println!();

                None
            })?;
        }

        Some(Subcmd::Init {
            init_duration_arg: InitDurationArg { duration },
            wait_after,
        }) => {
            init_night_light(duration, wait_after, cli.lenient)?;
        }

        Some(Subcmd::KeepIniting {
            stop,
            delay,
            init_duration_arg: InitDurationArg { duration },
        }) => 'subcmd_handler: {
            let stop_msg =
                unsafe { RegisterWindowMessageW(w!(r"{5dbd5965-0cd4-4fa5-8453-41e3871fd168}")) }
                    .nonzero_or_win32_err()?;

            //. Always replace a previous instance.
            unsafe { SendNotifyMessageW(HWND_BROADCAST, stop_msg, WPARAM(0), LPARAM(0))? };

            if stop {
                break 'subcmd_handler;
            }

            init_night_light(duration, false, cli.lenient)?;

            //. Remove console, if this is the only process using it.
            //. For cases where the process was started from a shortcut file or so and the console window shouldn't continue to linger around.
            if !has_shared_console()? {
                unsafe { FreeConsole()? };
            }

            let mut h_power_notify = None;
            let mut last_monitor_state = PowerMonitorOn;
            let startup_instant = Instant::now(); // To ignore first status message.

            try_then_favor_app_error(|| -> anyhow::Result<()> {
                let window_class = WindowClass::new(|hwnd, msg_id, wparam, lparam| {
                    match msg_id {
                        WM_CREATE => {
                            let success = try_or_quit_now(|| -> anyhow::Result<_> {
                                ctrlc::set_handler(move || {
                                    let _ = unsafe {
                                        PostMessageW(hwnd, stop_msg, WPARAM(0), LPARAM(0))
                                    };
                                })?;

                                h_power_notify = Some(
                                    ResGuard::with_acq_and_unregister_power_setting_notification(
                                        || unsafe {
                                            RegisterPowerSettingNotification(
                                                HANDLE(hwnd.0),
                                                //TODO: Use `GUID_SESSION_DISPLAY_STATUS` instead? See <https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids#guid_session_display_status>. (Mind other occurrences besides this one.)
                                                &GUID_CONSOLE_DISPLAY_STATE,
                                                //TODO: See <https://github.com/microsoft/win32metadata/issues/1779>.
                                                DEVICE_NOTIFY_WINDOW_HANDLE.0,
                                            )
                                        },
                                    )?,
                                );

                                Ok(())
                            })
                            .is_some();

                            Some(LRESULT(if success { 0 } else { -1 }))
                        }

                        WM_POWERBROADCAST => {
                            // Author's experience on Windows 10 in Dec. 2023: With a multi-monitor setup, `GUID_CONSOLE_DISPLAY_STATE` isn't sent when just one monitor changes its on-off state, while others stay active, but only when all monitors at once or the last active monitor is turned on/off. When turning just one monitor on/off, while others stay active, there are various other messages of unclear relevance, though, like, e.g., `WM_DEVICECHANGE`, `WM_DISPLAYCHANGE` and `WM_SETTINGCHANGE`. All of this wasn't a problem though, because the OS only failed to reapply the color temperature when a single active monitor was turned off and then turned on again.

                            match unsafe { translate_power_broadcast_msg(wparam, &lparam) } {
                                PowerBroadcastMsg::PowerSettingChange { setting } => {
                                    if setting.PowerSetting == GUID_CONSOLE_DISPLAY_STATE {
                                        try_or_quit_now(|| -> anyhow::Result<_> {
                                            let new_monitor_state = unsafe {
                                                *setting.cast_data::<MONITOR_DISPLAY_STATE>()?
                                            };

                                            if startup_instant.elapsed().as_millis() > 200
                                                && new_monitor_state != last_monitor_state
                                                && new_monitor_state == PowerMonitorOn
                                            {
                                                // Monitor just turned on.
                                                thread::sleep(Duration::from_millis(delay as _));
                                                if let Err(error) =
                                                    init_night_light(duration, false, cli.lenient)
                                                {
                                                    eprintln!("error: {error:?}");
                                                }
                                            }

                                            last_monitor_state = new_monitor_state;

                                            Ok(LRESULT(1))
                                        })
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        }

                        id if id == stop_msg => {
                            let _ = unsafe { DestroyWindow(hwnd) };
                            Some(LRESULT(0))
                        }

                        WM_DESTROY => {
                            drop(h_power_notify.take());
                            unsafe { PostQuitMessage(0) };
                            Some(LRESULT(0))
                        }

                        _ => None,
                    }
                })?;

                let _window = Window::new_invisible(&window_class)?;
                msg_loop::run()?;
                Ok(())
            })
            .map_err(|e| anyhow!(e))?;
        }

        Some(Subcmd::Cycle { gamma }) => {
            const NUM_CYCLES: usize = 6;
            const FRAME_DURATION: Duration = Duration::from_millis(54);
            const NUM_STEPS_PER_HALF_CYCLE: usize = 17;
            const STEP_SIZE: f32 = 1.0 / NUM_STEPS_PER_HALF_CYCLE as f32;
            let inverse_gamma = 1.0 / gamma.unwrap_or(1.0);

            // Make iterator.
            let cold_to_warm_iter =
                (0..=NUM_STEPS_PER_HALF_CYCLE).map(|i| (i as f32 * STEP_SIZE).powf(inverse_gamma));
            let warm_to_cold_iter = cold_to_warm_iter.clone().rev();
            let mut cycling_iter =
                iter::repeat(cold_to_warm_iter.skip(1).chain(warm_to_cold_iter.skip(1)))
                    .take(NUM_CYCLES)
                    .flatten();

            // Cycle.
            let orig_night_light = NightLight::from_reg()?;

            println!("Cycling Night Light for a couple of seconds...");

            let must_abort = Arc::new(Mutex::new(false));
            let moved_must_abort = must_abort.clone();
            ctrlc::set_handler(move || {
                *moved_must_abort.lock().unwrap() = true;
            })?;

            let cycle_result = cycling_iter.try_for_each(|warmth| {
                if *must_abort.lock().unwrap() {
                    Ok(())
                } else {
                    let mut night_light = NightLight::from_reg()?;

                    night_light.set_night_preview_active(true);
                    night_light.set_warmth(Some(warmth));

                    let result = night_light.write_to_reg();
                    thread::sleep(FRAME_DURATION);

                    result
                }
            });

            // Restore previous configuration.
            let mut night_light = NightLight::from_reg()?;
            night_light.set_night_preview_active(orig_night_light.night_preview_active());
            night_light.set_night_color_temp(orig_night_light.night_color_temp());
            night_light.write_to_reg()?;

            cycle_result?;
        }

        // Subcommands that need a common parsed `NightLight`.
        subcmd => {
            let mut night_light =
                NightLight::from_reg_with_strictness(Strictness::from_lenient_bool(cli.lenient))?;

            night_light.set_uses_12_hour_clock(cli.am_pm);

            let temp_args = match &subcmd {
                None => {
                    if cli.json {
                        println!("{}", night_light.to_json());
                    } else {
                        println!("{night_light}");
                        println!();
                        println!("{}", "Pass '--help' to see available actions.".dimmed());
                    }

                    None
                }

                Some(Subcmd::Switch {
                    on_off_args: RequiredOnOffArgs { toggle, on, .. },
                    temp_args,
                }) => {
                    night_light.set_active(if *toggle { !night_light.active() } else { *on });
                    Some(temp_args)
                }

                Some(Subcmd::Temp { temp_args }) => Some(temp_args),

                Some(Subcmd::Preview {
                    on_off_args: RequiredOnOffArgs { toggle, on, .. },
                    temp_args,
                }) => {
                    night_light.set_night_preview_active(if *toggle {
                        !night_light.night_preview_active()
                    } else {
                        *on
                    });
                    Some(temp_args)
                }

                Some(Subcmd::Schedule {
                    schedule_args:
                        ScheduleArgs {
                            on_off_args,
                            r#type,
                            night,
                            temp_args,
                        },
                }) => {
                    if let Some(on_off_args) = on_off_args {
                        night_light.set_schedule_active(if on_off_args.toggle {
                            !night_light.schedule_active()
                        } else {
                            on_off_args.on
                        });
                    }

                    if let Some(r#type) = r#type {
                        night_light.set_schedule_type(match r#type {
                            cli::ScheduleType::Explicit => night_light::ScheduleType::Explicit,
                            cli::ScheduleType::Sun => night_light::ScheduleType::SunsetToSunrise,
                        });
                    }

                    if let Some(night) = night {
                        night_light.set_scheduled_night(*night);
                    }

                    temp_args.as_ref()
                }

                _ => unreachable!(),
            };

            if let Some(TempArgs {
                kelvin,
                warmth,
                default_temp,
                gamma,
            }) = temp_args
            {
                if *default_temp {
                    night_light.set_night_color_temp(None);
                } else if let Some(kelvin) = kelvin {
                    night_light.set_night_color_temp(Some(*kelvin));
                } else if let Some(warmth) = warmth {
                    night_light.set_warmth(Some(warmth.powf(1.0 / gamma.unwrap_or(1.0))));
                }
            }

            night_light.write_to_reg()?;
        }
    }

    Ok(())
}

fn init_night_light(
    duration_millis: Option<u16>,
    wait_after: bool,
    lenient: bool,
) -> Result<(), night_light::Error> {
    NightLight::init_with_strictness(
        duration_millis
            .map(|millis| Duration::from_millis(millis as _))
            .unwrap_or(NightLight::REASONABLE_INIT_DELAY),
        wait_after,
        Strictness::from_lenient_bool(lenient),
    )
}

fn has_shared_console() -> windows::core::Result<bool> {
    //! Returns whether the current process shares the console with other processes - e.g., because it was spawned in a terminal in a non-detaching way.

    let mut process_ids = vec![0];
    dual_call(FirstCallExpectation::Ok, |getting_len| unsafe {
        if getting_len || process_ids.len() > 1 {
            GetConsoleProcessList(&mut process_ids)
                .nonzero_or_win32_err()
                .map(|len| {
                    if getting_len && len > 1 {
                        process_ids.resize(len as _, 0);
                    }
                })
        } else {
            Ok(())
        }
    })?;

    let current_id = unsafe { GetCurrentProcessId() };
    Ok(process_ids.iter().any(|id| *id != current_id))
}
