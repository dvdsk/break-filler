use std::sync::Mutex;
use std::time::Duration;

use break_filler::{time, Activity, Planner, Store};
use jiff::civil;
use tempfile::tempdir;

/// time mock is done via a global static, a problem when running
/// tests in parallel. This ensures that does not happen.
static TEST_ACTIVE: Mutex<()> = Mutex::new(());

fn setup_test(test_name: &str, activity_count: usize, end_hour: i8) -> Planner {
    let path = tempdir().unwrap().path().join(test_name);
    let store = Store::new(path).unwrap();

    let program_start = civil::time(12, 0, 0, 0);
    let work_duration = Duration::from_secs(25 * 60);
    let break_duration = Duration::from_secs(5 * 60);
    time::setup_mock_with(program_start, break_duration, work_duration);

    Planner {
        load: 1.0,
        store,
        activities: vec![Activity {
            description: "test".to_owned(),
            count: activity_count,
            needs_confirm: false,
        }],
        window: std::ops::Range {
            start: civil::time(12, 0, 0, 0),
            end: civil::time(end_hour, 0, 0, 0),
        },
        period: Some(work_duration + break_duration),
        program_start: time::zoned_now(),
        break_duration: Some(break_duration),
    }
}

#[test]
fn reminders2_breaks4() {
    let _guard = TEST_ACTIVE.lock();
    let planner = setup_test("reminders2_breaks4", 2, 14);

    // `12:25 break - 12:55 break - 13:25 break - 13:55 break `
    // ` reminder                     reminder                `

    time::next_break();
    println!("\nfirst break, should have a reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nsecond break, should have no reminder");
    assert_eq!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nthird break, should have a reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nlast break, should have no reminder");
    assert_eq!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();
}

#[test]
fn reminders1_breaks4() {
    let _guard = TEST_ACTIVE.lock();
    let planner = setup_test("reminders1_breaks4", 1, 14);

    // `12:25 break - 12:55 break - 13:25 break - 13:55 break `
    // ` reminder                     reminder                `

    time::next_break();
    println!("\nfirst break, should have no reminder");
    assert!(planner.reminder(false).unwrap().is_empty());
    time::break_ends();

    time::next_break();
    println!("\nsecond break, should have no reminder");
    assert!(planner.reminder(false).unwrap().is_empty());
    time::break_ends();

    time::next_break();
    println!("\nthird break, should have reminder");
    assert!(!planner.reminder(false).unwrap().is_empty());
    time::break_ends();

    time::next_break();
    println!("\nlast break, should have no reminder");
    assert!(planner.reminder(false).unwrap().is_empty());
    time::break_ends();
}

#[test]
fn reminders2_breaks12() {
    let _guard = TEST_ACTIVE.lock();
    let planner = setup_test("reminders2_breaks12", 2, 18);

    for i in 0..12 {
        time::next_break();
        println!("\nbreak {i}");
        let reminders = planner.reminder(false).unwrap();
        if i == 3 || i == 7 {
            assert!(!reminders.is_empty(), "should have a reminder");
        } else {
            assert!(reminders.is_empty(), "should be no reminders");
        }
        time::break_ends();
    }
}

#[test]
fn reminders_inf_breaks4() {
    let _guard = TEST_ACTIVE.lock();
    let planner = setup_test("reminders2_breaks4", usize::MAX, 14);

    // `12:25 break - 12:55 break - 13:25 break - 13:55 break `
    // ` reminder                     reminder                `

    time::next_break();
    println!("\nfirst break, should have a reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nsecond break, should have no reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nthird break, should have a reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();

    time::next_break();
    println!("\nlast break, should have no reminder");
    assert_ne!(planner.reminder(false).unwrap(), Vec::new());
    time::break_ends();
}

#[test]
fn recovers() {
    let _guard = TEST_ACTIVE.lock();

    let path = tempdir().unwrap().path().join("recovers");

    let work_duration = Duration::from_secs(25 * 60);
    let break_duration = Duration::from_secs(5 * 60);

    let new_planner = |store| Planner {
        load: 1.0,
        store,
        activities: vec![Activity {
            description: "test".to_owned(),
            count: 2,
            needs_confirm: false,
        }],
        window: std::ops::Range {
            start: civil::time(12, 0, 0, 0),
            end: civil::time(18, 0, 0, 0),
        },
        period: Some(work_duration + break_duration),
        program_start: time::zoned_now(),
        break_duration: Some(break_duration),
    };

    {
        time::setup_mock_with(
            civil::time(12, 0, 0, 0),
            break_duration,
            work_duration,
        );
        let store = Store::new(&path).unwrap();
        let planner = new_planner(store);
        for i in 0..6 {
            time::next_break();
            println!("\nbreak {i}");
            let reminders = planner.reminder(false).unwrap();
            if i == 3 {
                assert!(!reminders.is_empty(), "should have a reminder");
            } else {
                assert!(reminders.is_empty(), "should be no reminders");
            }
            time::break_ends();
        }
    }

    time::setup_mock_with(
        civil::time(15, 0, 0, 0),
        break_duration,
        work_duration,
    );
    let store = Store::new(&path).unwrap();
    let planner = new_planner(store);

    for i in 6..12 {
        time::next_break();
        println!("\nbreak {i}");
        let reminders = planner.reminder(false).unwrap();
        if i == 7 {
            assert!(!reminders.is_empty(), "should have a reminder");
        } else {
            assert!(reminders.is_empty(), "should be no reminders");
        }
        time::break_ends();
    }
}
