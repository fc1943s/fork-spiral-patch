use biturbo_spiral::lexer::Style;
use biturbo_spiral::theme::{Theme, theme_from_settings, themed};

const SYSTEM_LIGHT: fn() -> Theme = || Theme::Light;
const SYSTEM_DARK: fn() -> Theme = || Theme::Dark;

#[test]
fn fixed_theme_wins_over_system() {
    let dark = "{\n  \"Theme\": 1,\n  \"FollowSystemTheme\": false,\n}";
    let light = "{\n  \"Theme\": 0,\n  \"FollowSystemTheme\": false\n}";
    assert_eq!(theme_from_settings(Some(dark), SYSTEM_LIGHT), Theme::Dark);
    assert_eq!(theme_from_settings(Some(light), SYSTEM_DARK), Theme::Light);
}

#[test]
fn following_the_system_or_missing_settings_use_the_system_theme() {
    let follow = "{\"FollowSystemTheme\": true, \"Theme\": 0}";
    assert_eq!(theme_from_settings(Some(follow), SYSTEM_DARK), Theme::Dark);
    assert_eq!(theme_from_settings(Some("{}"), SYSTEM_DARK), Theme::Dark);
    assert_eq!(theme_from_settings(None, SYSTEM_LIGHT), Theme::Light);
}

#[test]
fn follow_system_key_does_not_match_theme_key() {
    let only_follow = "{\"FollowSystemTheme\": false}";
    assert_eq!(theme_from_settings(Some(only_follow), SYSTEM_DARK), Theme::Dark);
}

#[test]
fn dark_theme_moves_keywords_to_the_readable_style_only() {
    assert_eq!(themed(Style::Keyword, Theme::Dark), Style::Value);
    assert_eq!(themed(Style::Keyword, Theme::Light), Style::Keyword);
    for style in [Style::Comment, Style::String, Style::Type, Style::Command, Style::Number] {
        assert_eq!(themed(style, Theme::Dark), style);
        assert_eq!(themed(style, Theme::Light), style);
    }
}
