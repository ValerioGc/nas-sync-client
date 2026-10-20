use super::*;

use crate::test_support::Scratch;

#[test]
fn the_first_start_has_no_link_and_notifications_on() {
    let scratch = Scratch::new("settings");
    let settings = load(&scratch.0).unwrap();
    assert_eq!(settings, Settings::default());
    assert!(settings.link.is_none());
    assert!(settings.notifications);
    assert!(!settings.autostart);
    assert_eq!(settings.appearance.locale, "it");
}

#[test]
fn settings_are_kept_and_preferences_checked() {
    let scratch = Scratch::new("settings");
    let mut settings = Settings::default();
    settings
        .apply(Preferences {
            notifications: false,
            autostart: true,
            max_send_kbps: 512,
            max_recv_kbps: 0,
        })
        .unwrap();
    save(&scratch.0, &settings).unwrap();
    assert_eq!(load(&scratch.0).unwrap(), settings);
    assert_eq!(settings.preferences().max_send_kbps, 512);
    assert_eq!(
        settings.apply(Preferences {
            max_send_kbps: MAX_KBPS + 1,
            ..settings.preferences()
        }),
        Err(ClientError::InvalidSync)
    );
}

#[test]
fn only_the_known_languages_and_themes_are_valid() {
    assert!(Appearance::default().valid());
    for (locale, theme) in [("fr", "dark"), ("it", "blue")] {
        assert!(!Appearance {
            locale: locale.into(),
            theme: theme.into()
        }
        .valid());
    }
}
