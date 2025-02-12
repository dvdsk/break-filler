use break_filler::window_manager;

#[test]
fn is_not_empty() {
    // you are running this from a terminal right? so then
    // there should be at least one window
    assert!(!dbg!(window_manager::visible_windows()).unwrap().is_empty());
}
