use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

struct TestState {
    current: jiff::Zoned,
    break_duration: Duration,
    work_duration: Duration,
}

static TEST_STATE: Mutex<Option<TestState>> = Mutex::new(None);
static TESTING: AtomicBool = AtomicBool::new(false);

pub(crate) fn setup_mock_with(args: &crate::cli::TestArgs) {
    let now = jiff::Zoned::now();
    let program_start = now.with().time(args.program_start).build().unwrap();

    *TEST_STATE.try_lock().expect("should not yet be in use") = Some(TestState {
        current: program_start,
        break_duration: args.break_duration,
        work_duration: args.work_duration,
    });
    TESTING.store(true, Ordering::Relaxed);
}

pub(crate) fn zoned_now() -> jiff::Zoned {
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

pub(crate) fn next_break() {
    let mut state = TEST_STATE.lock().expect("nothing should panic");
    let state = state
        .as_mut()
        .expect("setup_mock_with should have run already");
    state.current += state.work_duration
}

pub(crate) fn break_ends() {
    let mut state = TEST_STATE.lock().expect("nothing should panic");
    let state = state
        .as_mut()
        .expect("setup_mock_with should have run already");
    state.current += state.break_duration
}
