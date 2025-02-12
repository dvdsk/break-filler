use core::ops::Range;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use itertools::Itertools;

use crate::Activity;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Run(RunArgs),
    /// do not connect to break-enforcer but simulate a run.
    Test(TestArgs),
    Install(RunArgs),
    Remove,
}

#[derive(Args, Clone)]
pub struct RunArgs {
    /// Activity to remind and frequency (multiple supported). Leaving
    /// out frequency means the activity is issued every break.
    ///
    /// Example: drink:3
    ///
    /// Activities that contain spaces should be surround with single
    /// quotes (') to make sure the shell sees them as one argument.
    ///
    /// Example: 'drink some water:2'
    ///
    /// If the activity needs to be rescheduled is its not marked as complete
    /// by the user prepend an exclamation mark (!).
    ///
    /// Example: '!eat:1'
    #[arg(short, long, value_parser = reminder_parser)]
    pub activity: Vec<Activity>,

    /// Start and end time in between which reminders should
    /// be issued:
    #[arg(short, long, value_parser = window_parser, default_value = "00:00..23:59")]
    pub window: Range<jiff::civil::Time>,

    /// The percentage the user will be behind the computer during the window
    /// as a number between 0 and 1.0
    #[arg(short, long, default_value = "1.0")]
    pub load: f32,

    /// If there is a visible window with this title then do not open a
    /// reminder unless this is the last possibility to issue the reminder
    ///
    /// note multiple are allowed
    #[arg(short, long)]
    pub apps_blocking_activity: Vec<String>,
}

#[derive(Args, Clone)]
pub struct TestArgs {
    #[command(flatten)]
    pub run_args: RunArgs,

    #[arg(short = 'o', long, value_parser = duration_parser)]
    pub work_duration: Duration,
    #[arg(short, long, value_parser = duration_parser)]
    pub break_duration: Duration,

    #[arg(short = 'r', long, value_parser = time_parser)]
    pub program_start: jiff::civil::Time,
    #[arg(short, long)]
    pub periods: usize,
}

fn reminder_parser(s: &str) -> Result<Activity, String> {
    let needs_confirm = s.chars().next().is_some_and(|c| c == '!');
    let s = s.trim_start_matches('!');

    if s.chars().filter(|c| *c == ':').count() > 1 {
        return Err(
            "Activity argument may only contain one colon (:)".to_owned()
        );
    }

    if let Some((description, count)) = s.split_once(':') {
        Ok(Activity {
            description: description.to_owned(),
            count: count
                .parse()
                .map_err(|e| format!("Could not parse count as number: {e}"))?,
            needs_confirm,
        })
    } else {
        Ok(Activity {
            description: s.to_owned(),
            count: usize::MAX,
            needs_confirm,
        })
    }
}

fn window_parser(s: &str) -> Result<Range<jiff::civil::Time>, String> {
    let range_tokens = s
        .chars()
        .tuple_windows()
        .filter(|(c1, c2)| *c1 == '.' && *c2 == '.')
        .count();
    if range_tokens != 1 {
        return Err("Range must contain exactly one occurence of ..".to_owned());
    }

    if let Some((start, end)) = s.split_once("..") {
        let start =
            jiff::civil::Time::strptime("%H:%M", start).map_err(|e| {
                format!(
                    "Could not parse start time, should be \
                in format: 12:34 (hh:mm). Parse error: {e}"
                )
            })?;
        let end = jiff::civil::Time::strptime("%H:%M", end).map_err(|e| {
            format!(
                "Could not parse end time, should be \
                in format: 12:34 (hh:mm). Parse error: {e}"
            )
        })?;

        Ok(start..end)
    } else {
        unreachable!("We checked that '..' occured once")
    }
}

fn duration_parser(s: &str) -> Result<Duration, String> {
    jiff::civil::Time::strptime("%H:%M", s)
        .map_err(|e| {
            format!(
                "Could not parse time, should be \
                in format: 12:34 (hh:mm). Parse error: {e}"
            )
        })
        .map(|t| t.hour() as u64 * 60 * 60 + t.minute() as u64 * 60)
        .map(Duration::from_secs)
}

fn time_parser(s: &str) -> Result<jiff::civil::Time, String> {
    jiff::civil::Time::strptime("%H:%M", s).map_err(|e| {
        format!(
            "Could not parse time, should be \
                in format: 12:34 (hh:mm). Parse error: {e}"
        )
    })
}
