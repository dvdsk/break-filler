use std::ops::Range;
use std::thread;
use std::time::{Duration, Instant};

use break_enforcer::StateUpdate;
use cli::TestArgs;
use color_eyre::eyre::Context;
use iced::futures::channel::mpsc;
use jiff::civil::Time;

pub mod cli;
pub mod time;
pub mod ui;
pub mod window_manager;

pub type Reminder = String;
#[dbstruct::dbstruct(db=sled)]
pub struct Store {
    /// when the reminder was last issued
    reminder_last_at: HashMap<Reminder, jiff::Zoned>,
    /// total amount the reminder has been issued since window start
    reminder_counts: HashMap<Reminder, usize>,

    /// if this was before the window start we wipe the
    /// reminder data and breaks
    #[dbstruct(Default)]
    pub last_check: jiff::Zoned,

    /// breaks since the window started
    #[dbstruct(Default)]
    breaks: usize,
}

pub struct Planner {
    pub load: f32,
    pub store: Store,
    pub activities: Vec<Activity>,
    pub window: Range<jiff::civil::Time>,
    pub period: Option<Duration>,
    pub break_duration: Option<Duration>,
    pub program_start: jiff::Zoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub description: String,
    pub count: usize,
    pub needs_confirm: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    BreakStarted,
    BreakEnded,
    ParameterChange {
        break_duration: Duration,
        work_duration: Duration,
    },
    Confirmed {
        activity: String,
        at: Instant,
    },
}

impl Planner {
    fn init_store(&self) -> color_eyre::Result<()> {
        let last_check = self
            .store
            .last_check()
            .get()
            .wrap_err("Could not get last check from db")?;
        if self.enabled() && self.window_start() > last_check {
            self.store
                .reminder_counts()
                .clear()
                .wrap_err("clearing reminder_counts")?;
            self.store
                .breaks()
                .set(&0)
                .wrap_err("clearing breaks had")?;
            self.store
                .last_check()
                .set(&time::zoned_now())
                .wrap_err("updating last_checked")?;
        }

        Ok(())
    }

    fn count_for(&self, description: &str) -> color_eyre::Result<usize> {
        if let Some(reminder_count) = self
            .store
            .reminder_counts()
            .get(description)
            .wrap_err("could not get value")?
        {
            Ok(reminder_count)
        } else {
            self.store
                .reminder_counts()
                .insert(description, &0)
                .wrap_err("could not insert missing value")?;
            Ok(0)
        }
    }

    fn enabled(&self) -> bool {
        let now = time::zoned_now();

        if self.window.start < self.window.end {
            // like 12:00..18:00
            self.window.contains(&now.time())
        } else {
            // like 23:00..01:00
            (self.window.start..Time::midnight()).contains(&now.time())
                || (Time::midnight()..self.window.end).contains(&now.time())
        }
    }

    fn period(&self) -> Duration {
        self.period.expect(
            "Parameters are set on subscribe, \
                thus before first break notification",
        )
    }

    // user story:
    // started at 10am, want reminders equally spaced but might skip some part of the
    // window. Then reminders should be issued immediately when resuming.
    // Reminders should be spaced if possible (not follow one another).
    //
    // implementation:
    // plan breaks from the last_reminder given. If now is larger then the
    // time a reminder should be issued according to that planning add the
    // reminder.
    pub fn reminder(
        &self,
        should_skip_if_reasonable: bool,
    ) -> color_eyre::Result<Vec<Activity>> {
        self.init_store().wrap_err("Could not init store")?;

        let mut res = Vec::new();
        if !self.enabled() {
            return Ok(res);
        }

        let mut can_skip_all = true;
        let is_first_break = self.store.breaks().get()? == 0;

        for activity in &self.activities {
            let remaining_reps =
                activity.count - self.count_for(&activity.description)?;
            if remaining_reps < 1 {
                continue;
            }
            dbg!(remaining_reps);

            // plan using `max(last reminder, program start, window_start)`
            // as reference
            let reference = self
                .last_reminder(&activity.description)?
                .zip(self.break_duration)
                // checked_add can fail if the time does not exist
                // (winter to summer time for example)
                .and_then(|(last, break_duration)| {
                    last.checked_add(break_duration).ok()
                })
                .unwrap_or(self.program_start.clone())
                .max(self.window_start());

            let relative_window =
                dbg!(self.window_remaining(&reference).mul_f32(self.load));
            let relative_future_breaks =
                dbg!(relative_window.div_duration_f32(self.period())).floor()
                    as usize;
            if dbg!(is_first_break)
                && dbg!(relative_future_breaks / 2) > dbg!(activity.count)
            {
                dbg!("EXIT FIRST BREAK");
                continue;
            }

            dbg!(relative_future_breaks, relative_window);

            let break_spacing = (relative_future_breaks) as f32
                / (remaining_reps.saturating_add(1)) as f32;
            let next_reminder_at = break_spacing;
            // let next_reminder_at = leftover_breaks - next_reminder_at;
            dbg!(next_reminder_at, break_spacing);

            let breaks_after_this = relative_future_breaks
                .saturating_sub(self.break_number_relative_to(&reference));
            if dbg!(breaks_after_this) == 2 && remaining_reps == 1 {
                continue;
            }
            if breaks_after_this < remaining_reps {
                can_skip_all = false;
                dbg!(breaks_after_this, remaining_reps, can_skip_all);
            }

            if next_reminder_at.floor() as usize
                <= dbg!(self.break_number_relative_to(&reference))
            {
                res.push(activity.clone());
            }
        }

        self.increment_total_breaks()?;

        dbg!(can_skip_all, should_skip_if_reasonable);
        if can_skip_all && should_skip_if_reasonable {
            return Ok(Vec::new());
        }

        for activity in &res {
            if !activity.needs_confirm {
                self.mark_completed(&activity.description)?;
            }
        }

        Ok(res)
    }

    fn increment_total_breaks(&self) -> color_eyre::Result<()> {
        let curr = self.store.breaks().get()?;
        self.store.breaks().set(&(curr + 1))?;
        Ok(())
    }

    fn break_number_relative_to(&self, reference: &jiff::Zoned) -> usize {
        let breaks_elapsed = reference
            .duration_until(dbg!(&time::zoned_now()))
            .unsigned_abs()
            .div_duration_f32(self.period())
            .floor() as usize;
        breaks_elapsed + 1
    }

    pub fn mark_completed(
        &self,
        description: &String,
    ) -> Result<(), color_eyre::eyre::Error> {
        if let Some(curr) = self
            .store
            .reminder_counts()
            .get(description)
            .wrap_err("getting count")?
        {
            self.store
                .reminder_counts()
                .insert(description, &(curr + 1))
                .wrap_err("setting count")?;
        } else {
            self.store
                .reminder_counts()
                .insert(description, &1)
                .wrap_err("setting count")?;
        }

        self.store
            .reminder_last_at()
            .insert(description, &time::zoned_now())
            .wrap_err("setting last at")?;
        Ok(())
    }

    fn last_reminder(
        &self,
        description: &str,
    ) -> color_eyre::Result<Option<jiff::Zoned>> {
        self.store
            .reminder_last_at()
            .get(description)
            .wrap_err("could not get last reminder at from db")
    }

    fn window_start(&self) -> jiff::Zoned {
        let now = time::zoned_now();
        if self.window.start <= now.time() {
            now.with()
                .time(self.window.start)
                .build()
                .unwrap_or_else(|_| {
                    panic!("time: {} does not exist today", self.window.start)
                })
        } else {
            now.with()
                .time(self.window.start)
                .build()
                .unwrap_or_else(|_| {
                    panic!("time: {} does not exist today", self.window.start)
                })
                .yesterday()
                .unwrap_or_else(|_| {
                    panic!(
                        "time: {} does not exist yesterday",
                        self.window.start
                    )
                })
        }
    }

    fn window_end(&self) -> jiff::Zoned {
        let now = time::zoned_now();
        if self.window.end >= now.time() {
            now.with()
                .time(self.window.end)
                .build()
                .unwrap_or_else(|_| {
                    panic!("time: {} does not exist today", self.window.end)
                })
        } else {
            now.with()
                .time(self.window.end)
                .build()
                .unwrap_or_else(|_| {
                    panic!("time: {} does not exist today", self.window.end)
                })
                .tomorrow()
                .unwrap_or_else(|_| {
                    panic!("time: {} does not exist tomorrow", self.window.end)
                })
        }
    }

    fn window_remaining(&self, reference: &jiff::Zoned) -> Duration {
        dbg!(reference)
            .duration_until(&dbg!(self.window_end()))
            .unsigned_abs()
    }
}

pub fn spawn_mock_break_enforcer_interface(test_config: TestArgs) {
    let (mut tx, rx) = mpsc::channel(64);
    thread::spawn(move || {
        tx.try_send(Message::ParameterChange {
            break_duration: test_config.break_duration,
            work_duration: test_config.work_duration,
        })
        .unwrap();
        thread::sleep(Duration::from_millis(250));

        for i in 0..test_config.periods {
            eprintln!("sending break start {i}");
            tx.try_send(Message::BreakStarted).unwrap();
            thread::sleep(Duration::from_secs(1));
            time::break_ends();
            time::next_break();
            tx.try_send(Message::BreakEnded).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });
    ui::send_rx(rx);
}

pub fn spawn_break_enforcer_interface() {
    let (mut tx, rx) = mpsc::channel(64);
    thread::spawn(move || {
        let mut api = break_enforcer::ReconnectingApi::new().subscribe();
        loop {
            match api.recv_update() {
                StateUpdate::ParameterChange {
                    break_duration,
                    work_duration,
                } => tx
                    .try_send(Message::ParameterChange {
                        break_duration,
                        work_duration,
                    })
                    .expect("cant lag so much that message can not be send"),
                StateUpdate::BreakStarted => {
                    tx.try_send(Message::BreakStarted).expect(
                        "cant lag so much that message can not be send",
                    );
                }
                StateUpdate::BreakEnded => todo!(),
                StateUpdate::WentIdle => todo!(),
                StateUpdate::Reset => todo!(),
            }
        }
    });
    ui::send_rx(rx);
}
