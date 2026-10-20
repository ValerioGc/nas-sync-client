use super::*;

#[test]
fn the_summary_opens_next_to_the_icon_and_inside_the_screen() {
    let screen = (0.0, 0.0, 1920.0, 1040.0);
    let size = (360.0, 480.0);
    assert_eq!(
        mini_position(screen, size, Some((1800.0, 1045.0, 24.0, 24.0))),
        (1552.0, 552.0)
    );
    assert_eq!(
        mini_position(screen, size, Some((900.0, 0.0, 24.0, 24.0))),
        (732.0, 32.0)
    );
    assert_eq!(mini_position(screen, size, None), (1552.0, 552.0));
    assert_eq!(
        mini_position(screen, size, Some((0.0, 1045.0, 24.0, 24.0))),
        (8.0, 552.0)
    );
}

#[test]
fn every_state_has_its_icon_and_label_in_both_languages() {
    for status in [
        Status::Unconfigured,
        Status::Paused,
        Status::Disconnected,
        Status::Syncing,
        Status::Synced,
        Status::Error,
    ] {
        assert!(icon(status).is_some());
        assert_ne!((labels("it").status)(status), (labels("en").status)(status));
    }
    assert_eq!(labels("it").pause, "Sospendi sincronizzazione");
    assert_eq!(labels("en").resume, "Resume sync");
    assert_eq!(labels("fr").quit, "Esci");
}
