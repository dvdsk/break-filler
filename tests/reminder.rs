use std::time::Duration;

use break_filler::{time, Activity, Planner, Store};
use jiff::civil;
use tempfile::tempdir;

#[test]
fn reminders2_breaks4() {
    let path = tempdir().unwrap().path().join("test.db");
    let store = Store::new(path).unwrap();

    let program_start = civil::time(12, 0, 0, 0);
    let work_duration = Duration::from_secs(25 * 60);
    let break_duration = Duration::from_secs(5 * 60);
    time::setup_mock_with(program_start, break_duration, work_duration);

    let planner = Planner {
        load: 1.0,
        store,
        activities: vec![Activity {
            description: "test".to_owned(),
            count: 2,
        }],
        window: std::ops::Range {
            start: civil::time(12, 0, 0, 0),
            end: civil::time(14, 0, 0, 0),
        },
        period: Some(work_duration + break_duration),
        program_start: time::zoned_now(),
    };

    // `12:25 break - 12:55 break - 13:25 break - 13:55 break `
    // ` reminder                     reminder                `

    time::next_break();
    println!("\nfirst break, should have a reminder");
    assert_ne!(planner.reminder().unwrap(), Vec::<String>::new());
    time::break_ends();

    time::next_break();
    println!("\nsecond break, should have no reminder");
    assert_eq!(planner.reminder().unwrap(), Vec::<String>::new());
    time::break_ends();

    time::next_break();
    println!("\nthird break, should have a reminder");
    assert_ne!(planner.reminder().unwrap(), Vec::<String>::new());
    time::break_ends();

    time::next_break();
    println!("\nlast break, should have no reminder");
    assert_eq!(planner.reminder().unwrap(), Vec::<String>::new());
    time::break_ends();
}

#[test]
fn reminders1_breaks4() {
    let path = tempdir().unwrap().path().join("test.db");
    let store = Store::new(path).unwrap();

    let program_start = civil::time(12, 0, 0, 0);
    let work_duration = Duration::from_secs(25 * 60);
    let break_duration = Duration::from_secs(5 * 60);
    time::setup_mock_with(program_start, break_duration, work_duration);

    let planner = Planner {
        load: 1.0,
        store,
        activities: vec![Activity {
            description: "test".to_owned(),
            count: 1,
        }],
        window: std::ops::Range {
            start: civil::time(12, 0, 0, 0),
            end: civil::time(14, 0, 0, 0),
        },
        period: Some(work_duration + break_duration),
        program_start: time::zoned_now(),
    };

    // `12:25 break - 12:55 break - 13:25 break - 13:55 break `
    // ` reminder                     reminder                `

    time::next_break();
    println!("\nfirst break, should have no reminder");
    assert!(planner.reminder().unwrap().is_empty());
    time::break_ends();

    time::next_break();
    println!("\nsecond break, should have reminder");
    assert_eq!(planner.reminder().unwrap(), vec!["test".to_owned()]);
    time::break_ends();

    time::next_break();
    println!("\nthird break, should have no reminder");
    assert!(planner.reminder().unwrap().is_empty());
    time::break_ends();

    time::next_break();
    println!("\nlast break, should have no reminder");
    assert!(planner.reminder().unwrap().is_empty());
    time::break_ends();
}

#[test]
fn reminders2_breaks12() {
    let path = tempdir().unwrap().path().join("test.db");
    let store = Store::new(path).unwrap();

    let program_start = civil::time(12, 0, 0, 0);
    let work_duration = Duration::from_secs(25 * 60);
    let break_duration = Duration::from_secs(5 * 60);
    time::setup_mock_with(program_start, break_duration, work_duration);

    let planner = Planner {
        load: 1.0,
        store,
        activities: vec![Activity {
            description: "test".to_owned(),
            count: 2,
        }],
        window: std::ops::Range {
            start: civil::time(12, 0, 0, 0),
            end: civil::time(18, 0, 0, 0),
        },
        period: Some(work_duration + break_duration),
        program_start: time::zoned_now(),
    };

    for i in 0..12 {
        time::next_break();
        println!("\nbreak {i}");
        let reminders = planner.reminder().unwrap();
        if i == 4 || i == 8 {
            assert!(!reminders.is_empty(), "should have a reminder");
        } else {
            assert!(reminders.is_empty(), "should be no reminders");
        }
        time::break_ends();
    }
}
