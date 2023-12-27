use std::str::FromStr;

use sem_reg::cloud_store::night_light::ClockTimeFrame;

const DEFAULT_GAMMA: &str = "1.6";

#[derive(clap::Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"), version)]
pub struct Cli {
    /// Show 12-hour instead of 24-hour clock times in most important places.
    #[arg(short = 'm', long, visible_alias = "12")]
    pub am_pm: bool,

    /// Be less strict when handling the registry values. Required when at least one of them doesn't exist. Generally to be avoided.
    #[arg(short, long)]
    pub lenient: bool,

    /// Print current configuration as JSON for consumation by software.
    #[arg(short, long)]
    pub json: bool,

    #[command(subcommand)]
    pub subcmd: Option<Subcmd>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcmd {
    /// Initialize Night Light after OS log-on or when turning the screen back on.
    ///
    /// This command briefly activates preview mode in a way that should be invisible.
    ///
    /// Always run this command or 'keep-initing' after logging on to Windows - e.g., by creating an autostart entry or by running it first in your automation script. If you don't, your possibilities of getting a color temperature effectively applied are limited. It particularly won't be applied, if Night Light is already active and you try to change the temperature.
    ///
    /// If, after turning your turned-off screen back on, you exprerience that the color temperature is cold again, this command should also restore the configured temperature.
    Init {
        #[command(flatten)]
        init_duration_arg: InitDurationArg,

        /// Additionally wait the same duration after the last action. If you must run another command in a script in direct succession, use this switch to ensure the Night Light engine doesn't miss a registry value. In the worst case, the engine missing a value can lead to Night Light becoming broken until restart or at least log-off.
        #[arg(short = 'a', long)]
        wait_after: bool,
    },

    /// Keeps running and maintains the color temperature.
    ///
    /// Performs the 'init' command initially and whenever the first screen was turned back on (i.e., after all have been turned off). If you find that, after turning the screen back on, the configured color temperature doesn't apply anymore, this command should correct that problem automatically in every case by running in the background. You can, e.g., create an autostart entry to run it.
    #[command(visible_alias = "keep")]
    KeepIniting {
        /// Simply stops a possibly running instance of this app that was also run with this command, and then ends.
        #[arg(short, long, conflicts_with_all = ["delay", "duration"])]
        stop: bool,

        /// The number of milliseconds to delay the 'init' command after receiving the information that the screen was turned on. Too small values can prevent the command from working or make it unreliable. Omit the switch to use the default value.
        #[arg(short = 'l', long, default_value = "100")]
        delay: u16,

        #[command(flatten)]
        init_duration_arg: InitDurationArg,
    },

    /// Switch Night Light on or off.
    ///
    /// When switching off, it's advisable to also set cold color temperature (in same command) to prevent a strange transition when turning it on again with less warmth at a later time.
    ///
    /// When used together with a schedule change in direct succession, use a delay in between and switch last.
    #[command(visible_alias = "sw")]
    Switch {
        #[command(flatten)]
        on_off_args: RequiredOnOffArgs,

        #[command(flatten)]
        temp_args: TempArgs,
    },

    /// Adjust color temperature on its own.
    ///
    /// Note that you can also adjust it in other commands.
    #[command(visible_aliases = ["t"])]
    Temp {
        #[command(flatten)]
        temp_args: TempArgs,
    },

    /// Turn preview mode on or off.
    ///
    /// This is the mode that's activated while moving the slider in the official settings. Makes for a hard transition instead of a smooth one. Should not be left on, because it blocks other changes.
    #[command(visible_aliases = ["p", "prev"])]
    Preview {
        #[command(flatten)]
        on_off_args: RequiredOnOffArgs,

        #[command(flatten)]
        temp_args: TempArgs,
    },

    /// Cycles a few times between cold and warm color temperature.
    ///
    /// Aids in finding a suitable gamma value for other commands by cycling the warmth factor from 0 to 1 and back. Display a white surface on the screen and pay attention to the perceived uniformity of the color temperatures. To be able to form a balanced opinion, it's recommended to also manually change the warmth factor in 0.1 steps and verify the step from 0 to the smallest value you plan to use.
    Cycle {
        /// See other commands like 'temp' for an explanation.
        #[arg(short, long, num_args = 0..=1, default_missing_value = DEFAULT_GAMMA, value_parser = gamma_value_parser)]
        gamma: Option<f32>,
    },

    /// Configure the schedule.
    #[command(visible_alias = "sch")]
    Schedule {
        #[command(flatten)]
        schedule_args: ScheduleArgs,
    },

    /// Export registry values to .reg file.
    #[command(visible_alias = "exp")]
    Export {
        /// The file path to use. Should have .reg extension. If not specified, defaults to filename based on current local time.
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Delete Night Light registry values to reset the feature. Requires log-off/restart.
    ///
    /// Useful in case the values became corrupted for any reason, leaving the feature in an unusable state. After deletion, you should restart or at least log-off.
    #[command(visible_alias = "del")]
    Delete,

    /// Monitor Night Light registry values for external changes, displaying technical details.
    #[command(visible_alias = "mon")]
    Monitor,
}

#[derive(clap::Args, Debug)]
pub struct InitDurationArg {
    /// The number of milliseconds to block while holding preview mode active. Only use this, if you really must customize the waiting time. Too short of a duration may possibly temporarily break Night Light.
    #[arg(short, long)]
    pub duration: Option<u16>,
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct RequiredOnOffArgs {
    #[arg(short = '1', long)]
    pub on: bool,

    #[arg(short = '0', long)]
    pub off: bool,

    #[arg(short, long)]
    pub toggle: bool,
}

#[derive(clap::Args, Debug)]
#[group(multiple = false)]
pub struct OnOffArgs {
    #[arg(short = '1', long)]
    pub on: bool,

    #[arg(short = '0', long)]
    pub off: bool,

    #[arg(short, long)]
    pub toggle: bool,
}

#[derive(clap::Args, Debug)]
pub struct ScheduleArgs {
    #[command(flatten)]
    pub on_off_args: Option<OnOffArgs>,

    /// Whether the explicit or the sunset-to-sunrise schedule should be in effect. Effectiveness of the latter also depends on the state of location services.
    #[arg(short = 'T', long)]
    pub r#type: Option<ScheduleType>,

    /// Start and end time for explicit schedule type. The times work exact to the minute, even if not displayed with this accuracy in the official settings. Examples for valid values: '20:21-6:00', '08:00pm-05:45AM', '9:59-9:59am'.
    #[arg(short, long)]
    pub night: Option<ClockTimeFrame>,

    #[command(flatten)]
    pub temp_args: Option<TempArgs>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ScheduleType {
    #[value(aliases = ["e", "ex", "exp", "expl"])]
    Explicit,
    Sun,
}

#[derive(clap::Args, Debug)]
pub struct TempArgs {
    /// Night time color temperature in Kelvin.
    #[arg(short, long, conflicts_with_all = ["warmth", "default_temp", "gamma"])]
    pub kelvin: Option<u16>,

    /// Kelvin value expressed as an inversely proportional factor from 0.0 to 1.0. Steps in the upper range are perceived as more intense, which is why they should be smaller to achieve the same step in perception as larger steps in the lower range. You can also use '--gamma' for this correction with this switch.
    #[arg(short, long, conflicts_with_all = ["kelvin", "default_temp"])]
    pub warmth: Option<f32>,

    /// The gamma exponent whose inverse is applied to '--warmth'. When omitting the number, a default is used. When omitting the switch, gamma correction isn't applied.
    #[arg(short, long, num_args = 0..=1, default_missing_value = DEFAULT_GAMMA, value_parser = gamma_value_parser, requires = "warmth")]
    pub gamma: Option<f32>,

    /// Apply Night Light's default color temperature.
    #[arg(short, long, conflicts_with_all = ["kelvin", "warmth", "gamma"])]
    pub default_temp: bool,
}

fn gamma_value_parser(string: &str) -> Result<f32, String> {
    let gamma = f32::from_str(string).map_err(|e| e.to_string())?;

    if gamma >= 1.0 && gamma <= 3.0 {
        Ok(gamma)
    } else {
        Err("value out of range 1.0..=3.0".to_string())
    }
}
