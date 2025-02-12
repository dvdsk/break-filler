use std::sync::Mutex;
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::futures::Stream;
use iced::widget::Container;
use iced::Length::Fill;
use iced::{widget, Alignment, Theme};
use iced::{window, Element, Subscription, Task};

use crate::cli::RunArgs;
use crate::{time, window_manager, Activity, Message, Planner, Store};

pub struct Ui {
    planner: Planner,
    error: Option<color_eyre::Report>,
    active_theme: Theme,
    active_window: Option<window::Id>,
    active_reminders: Vec<DisplayedActivity>,
    skip_when_visible: Vec<String>,
}

struct DisplayedActivity {
    description: String,
    checkbox: Option<bool>,
}

impl Ui {
    pub const FONT: &'static [u8] =
        include_bytes!("../fonts/Poppins-Medium.ttx");

    pub fn new(
        RunArgs {
            activity,
            window: deadline,
            skip_when_visible: apps_blocking_activity,
            load,
        }: RunArgs,
        store: Store,
    ) -> (Self, Task<Message>) {
        (
            Ui {
                active_theme: Theme::TokyoNight,
                active_window: None,
                active_reminders: Vec::new(),
                skip_when_visible: apps_blocking_activity,
                planner: Planner {
                    store,
                    activities: activity,
                    window: deadline,
                    load,
                    period: None,
                    program_start: time::zoned_now(),
                    break_duration: None,
                },
                error: None,
            },
            Task::none(),
        )
    }

    pub fn title(&self, _: window::Id) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    pub fn update_or_error(
        &mut self,
        message: Message,
    ) -> color_eyre::Result<Task<Message>> {
        Ok(match &message {
            Message::ParameterChange {
                break_duration,
                work_duration,
            } => {
                self.planner.period = Some(*break_duration + *work_duration);
                Task::none()
            }
            Message::BreakStarted => {
                if self.active_window.is_some() {
                    return Ok(Task::none());
                }

                self.update_active_reminders()?;
                self.active_theme = self.update_theme();

                if self.active_reminders.is_empty() {
                    Task::none()
                } else {
                    eprintln!("got break start, opening window");
                    let (id, task) = window::open(window::Settings::default());
                    self.active_window = Some(id);
                    task.discard()
                }
            }
            Message::BreakEnded => {
                self.active_reminders = self
                    .active_reminders
                    .drain(..)
                    .filter(|DisplayedActivity { checkbox, .. }| {
                        checkbox.is_some()
                    })
                    .collect();

                if let Some(id) = self.active_window {
                    if self.active_reminders.is_empty() {
                        self.active_window = None;
                        return Ok(window::close(id));
                    }
                }
                Task::none()
            }
            Message::Confirmed { activity, at } => {
                let Some(to_remove) = self
                    .active_reminders
                    .iter()
                    .position(|act| &act.description == activity)
                else {
                    return Ok(Task::none());
                };

                self.active_reminders[to_remove].checkbox = Some(true);
                let sleep_left =
                    Duration::from_millis(300).saturating_sub(at.elapsed());
                if !sleep_left.is_zero() {
                    return Ok(Task::future(resend_later(message, sleep_left)));
                }

                self.active_reminders.swap_remove(to_remove);
                self.planner.mark_completed(&activity)?;

                if let Some(id) = self.active_window {
                    if self.active_reminders.is_empty() {
                        self.active_window = None;
                        return Ok(window::close(id));
                    }
                }
                Task::none()
            }
        })
    }

    fn update_active_reminders(
        &mut self,
    ) -> Result<(), color_eyre::eyre::Error> {
        let should_skip_if_reasonable =
            window_manager::visible_windows().into_iter().any(|window| {
                self.skip_when_visible.iter().any(|app| {
                    window.to_lowercase().contains(&app.to_lowercase())
                })
            });
        self.active_reminders = self
            .planner
            .reminder(should_skip_if_reasonable)?
            .into_iter()
            .map(
                |Activity {
                     description,
                     needs_confirm,
                     ..
                 }| DisplayedActivity {
                    description,
                    checkbox: needs_confirm.then(|| false),
                },
            )
            .collect();
        Ok(())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let error_to_display = match self.update_or_error(message) {
            Ok(task) => return task,
            Err(e) => e,
        };

        self.error = Some(error_to_display);
        if self.active_window.is_none() {
            let (id, task) = window::open(window::Settings::default());
            self.active_window = Some(id);
            task.discard()
        } else {
            Task::none()
        }
    }

    pub fn view(&self, _: window::Id) -> Element<Message> {
        if let Some(error) = &self.error {
            let error = format!("{:?}", error);
            return widget::text(error).into();
        }

        let column = widget::column(self.active_reminders.iter().map(
            |DisplayedActivity {
                 description,
                 checkbox: needs_confirm,
             }| {
                if let Some(checked) = needs_confirm {
                    widget::checkbox(description.clone(), *checked)
                        .text_size(80)
                        .size(80)
                        .on_toggle(|_| Message::Confirmed {
                            activity: description.clone(),
                            at: Instant::now(),
                        })
                        .into()
                } else {
                    widget::text(description)
                        .size(80)
                        .align_x(Alignment::Center)
                        .into()
                }
            },
        ))
        .spacing(40)
        .align_x(Alignment::Center)
        .width(Fill);

        Container::new(column).center(Fill).into()
    }

    fn update_theme(&mut self) -> Theme {
        match dark_light::detect() {
            Ok(dark_light::Mode::Dark) => Theme::TokyoNight,
            Ok(dark_light::Mode::Light) => Theme::SolarizedLight,
            Ok(dark_light::Mode::Unspecified) => Theme::SolarizedLight,
            Err(e) => {
                eprintln!(
                    "Could not detect if system dark mode on, error: {e:?}"
                );
                Theme::SolarizedLight
            }
        }
    }

    pub fn theme(&self, _: window::Id) -> Theme {
        self.active_theme.clone()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // never not call this, if you do the stream with break-enforcer
        // is ended and it can not be restart (program will crash attempting that)
        Subscription::run(take_global_stream)
    }
}

async fn resend_later(msg: Message, delay: Duration) -> Message {
    tokio::time::sleep(delay).await;
    msg
}

// forgive me for this sin:
// There is no way easy way to get data into an Iced subscription this seemed
// the easiest.
static GLOBAL_STREAM: Mutex<Option<mpsc::Receiver<Message>>> = Mutex::new(None);

pub fn send_rx(rx: mpsc::Receiver<Message>) {
    *GLOBAL_STREAM
        .try_lock()
        .expect("nothing has locked this yet") = Some(rx);
}

fn take_global_stream() -> impl Stream<Item = Message> {
    let rx = GLOBAL_STREAM
        .try_lock()
        .expect("called after lock is released by main")
        .take()
        .expect("called once");

    rx
}
