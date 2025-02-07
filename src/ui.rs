use std::sync::Mutex;

use iced::futures::channel::mpsc;
use iced::futures::Stream;
use iced::widget;
use iced::{window, Element, Subscription, Task};

use crate::cli::RunArgs;
use crate::{time, Message, Planner, Store};

pub struct Ui {
    planner: Planner,
    active_window: Option<window::Id>,
    active_reminders: Vec<String>,
}

impl Ui {
    pub fn new(
        RunArgs {
            activity,
            window: deadline,
        }: RunArgs,
        store: Store,
    ) -> (Self, Task<Message>) {
        (
            Ui {
                active_window: None,
                active_reminders: Vec::new(),
                planner: Planner {
                    store,
                    activities: activity,
                    window: deadline,
                    load: 0.5,
                    period: None,
                    program_start: time::zoned_now(),
                },
            },
            Task::none(),
        )
    }

    pub fn title(&self, _: window::Id) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match dbg!(message) {
            Message::ParameterChange {
                break_duration,
                work_duration,
            } => {
                self.planner.period = Some(break_duration + work_duration);
                Task::none()
            }
            Message::BreakStarted => {
                self.active_reminders = self.planner.reminder().unwrap();

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
                if let Some(id) = self.active_window {
                    window::close(id)
                } else {
                    Task::none()
                }
            }
        }
    }
    pub fn view(&self, _: window::Id) -> Element<Message> {
        widget::column(
            self.active_reminders
                .iter()
                .map(widget::text)
                .map(Element::from),
        )
        .into()
    }

    pub fn subscription(_: &Ui) -> Subscription<Message> {
        // never not call this, if you do the stream with break-enforcer
        // is ended and it can not be restart (program will crash attempting that)
        Subscription::run(take_global_stream)
    }
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
