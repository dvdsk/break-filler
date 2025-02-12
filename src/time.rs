use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use jiff::civil::Time;

struct TestState {
    current: jiff::Zoned,
    break_duration: Duration,
    work_duration: Duration,
}

static TEST_STATE: Mutex<Option<TestState>> = Mutex::new(None);
static TESTING: AtomicBool = AtomicBool::new(false);

pub fn setup_mock_from_args(args: &crate::cli::TestArgs) {
    setup_mock_with(
        args.program_start,
        args.break_duration,
        args.work_duration,
    );
}

pub fn setup_mock_with(
    program_start: Time,
    break_duration: Duration,
    work_duration: Duration,
) {
    let now = jiff::Zoned::now();
    let program_start = now.with().time(program_start).build().unwrap();

    *TEST_STATE.try_lock().expect("should not yet be in use") =
        Some(TestState {
            current: program_start,
            break_duration,
            work_duration,
        });
    TESTING.store(true, Ordering::Relaxed);
}

pub fn zoned_now() -> jiff::Zoned {
    if TESTING.load(Ordering::Relaxed) {
        TEST_STATE
            .lock()
            .expect("nothing should panic")
            .as_ref()
            .expect("setup_mock_with should have run already")
            .current
            .clone()
    } else {
        jiff::Zoned::now()
    }
}

pub fn next_break() {
    let mut state = TEST_STATE.lock().expect("nothing should panic");
    let state = state
        .as_mut()
        .expect("setup_mock_with should have run already");
    state.current += state.work_duration
}

pub fn break_ends() {
    let mut state = TEST_STATE.lock().expect("nothing should panic");
    let state = state
        .as_mut()
        .expect("setup_mock_with should have run already");
    state.current += state.break_duration
}
