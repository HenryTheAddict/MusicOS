use crate::app::Focus;
use crate::audio::EqPreset;
use crate::renderer::Canvas;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{figma_list_row, FontWeight};

pub fn draw_settings(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let _ = lang;
    let transition_t = ease_out_cubic(transition_t);
    let _ = theme;
    crate::ui::rows_preview(canvas);

    let items = [
        (Focus::SettingsGeneral, "General", 84.0),
        (Focus::SettingsAudio, "Audio", 202.0),
        (Focus::SettingsLooks, "Appearance", 320.0),
        (Focus::SettingsConnectivity, "Library Scan", 438.0),
        (Focus::SettingsUser, "USB Import", 556.0),
        (Focus::SettingsGetUpdate, "Updates", 674.0),
        (Focus::SettingsAbout, "About", 792.0),
    ];

    for (index, (item_focus, label, _)) in items.into_iter().enumerate() {
        let y = 68.0 + index as f32 * 112.0;
        let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.78));
        let x = 24.0 - (1.0 - row_t) * 64.0;
        let selected = matches!(
            (focus, item_focus),
            (Focus::SettingsTheme, Focus::SettingsLooks)
                | (Focus::SettingsLanguage, Focus::SettingsGeneral)
        ) || focus == item_focus;
        figma_list_row(canvas, x, y, 600.0, 104.0, selected, seconds);
        crate::ui::text_fit(
            canvas,
            label,
            x + 28.0,
            y + 18.0,
            480.0,
            46.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        canvas.text_right(
            "›",
            x + 564.0,
            y + 21.0,
            46.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Bold,
        );
    }
    let (title, description) = match focus {
        Focus::SettingsAudio => ("Audio", "Adjust sound and playback."),
        Focus::SettingsLooks | Focus::SettingsTheme => ("Appearance", "Choose a color theme."),
        Focus::SettingsConnectivity => ("Library Scan", "Find new music on your device."),
        Focus::SettingsUser => ("USB Import", "Import from connected storage."),
        Focus::SettingsGetUpdate => ("Updates", "Install a handheld software update."),
        Focus::SettingsAbout => ("About", "View device and library information."),
        _ => ("General", "Change your language, clock and display."),
    };
    crate::ui::preview_heading(canvas, title, description);
}

pub fn draw_settings_detail(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    brightness: u8,
    volume: u8,
    eq_preset: EqPreset,
    theme: ThemeName,
    battery_text: &str,
    seconds: f32,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    crate::ui::rows_preview(canvas);
    let (title, description) = match focus {
        Focus::SettingsLanguage => (
            "Language",
            "Choose the language used on this device.".to_owned(),
        ),
        Focus::SettingsDateTime => ("Date & Time", "Set the clock and calendar.".to_owned()),
        Focus::SettingsBrightness => (
            "Brightness",
            format!("{brightness}% · Left or Right to adjust"),
        ),
        Focus::SettingsVolume => ("Volume", format!("{volume}% · Left or Right to adjust")),
        Focus::SettingsEq => (
            "Equalizer",
            format!("{} · Left or Right to adjust", eq_preset.label()),
        ),
        _ => ("Theme", theme_label(theme).to_owned()),
    };
    crate::ui::preview_heading(canvas, title, &description);

    detail_row(
        canvas,
        95.0 + detail_offset(transition_t, 0),
        "Language",
        focus == Focus::SettingsLanguage,
        seconds,
        0.0,
    );
    detail_row(
        canvas,
        214.0 + detail_offset(transition_t, 1),
        "Date And Time",
        focus == Focus::SettingsDateTime,
        seconds,
        0.35,
    );
    detail_row(
        canvas,
        333.0 + detail_offset(transition_t, 2),
        &format!("Brightness {brightness}%"),
        focus == Focus::SettingsBrightness,
        seconds,
        0.7,
    );
    detail_row(
        canvas,
        452.0 + detail_offset(transition_t, 3),
        &format!("Volume {volume}%"),
        focus == Focus::SettingsVolume,
        seconds,
        1.05,
    );
    detail_row(
        canvas,
        571.0 + detail_offset(transition_t, 4),
        &format!("EQ {}", eq_preset.label()),
        focus == Focus::SettingsEq,
        seconds,
        1.4,
    );
    detail_row(
        canvas,
        690.0 + detail_offset(transition_t, 5),
        &format!("Theme {}", theme_label(theme)),
        focus == Focus::SettingsSwitchObject,
        seconds,
        1.75,
    );

    canvas.text_centered(
        battery_text,
        954.0,
        790.0,
        26.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
}

pub fn draw_date_time_setup(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    hour: u8,
    minute: u8,
    month: u8,
    day: u8,
    year: u16,
    _theme: ThemeName,
    seconds: f32,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    crate::ui::rows_preview(canvas);
    crate::ui::preview_heading(canvas, "Date & Time", "Used for history, sorting, and logs");
    canvas.text_centered(
        &format!("{hour:02}:{minute:02}"),
        954.0,
        328.0,
        114.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    canvas.text_centered(
        &format!("{year:04}-{month:02}-{day:02}"),
        954.0,
        482.0,
        44.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Regular,
    );
    for (index, (item_focus, label)) in [
        (Focus::DateHour, format!("Hour    {hour:02}")),
        (Focus::DateMinute, format!("Minute    {minute:02}")),
        (Focus::DateMonth, format!("Month    {month:02}")),
        (Focus::DateDay, format!("Day    {day:02}")),
        (Focus::DateYear, format!("Year    {year:04}")),
        (Focus::DateDone, "Done".to_owned()),
    ]
    .into_iter()
    .enumerate()
    {
        detail_row(
            canvas,
            88.0 + index as f32 * 120.0 + detail_offset(transition_t, index),
            &label,
            focus == item_focus,
            seconds,
            0.0,
        );
    }
}

fn detail_offset(transition_t: f32, index: usize) -> f32 {
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.82));
    (1.0 - row_t) * 42.0
}

fn theme_label(theme: ThemeName) -> &'static str {
    match theme {
        ThemeName::Seaglass => "Seaglass",
        ThemeName::Dark => "Dark",
        ThemeName::Blue => "Blue",
        ThemeName::Red => "Red",
        ThemeName::Green => "Green",
        ThemeName::Purple => "Purple",
    }
}

fn detail_row(
    canvas: &mut Canvas<'_>,
    y: f32,
    label: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    let _ = phase;
    let h = 108.0;
    figma_list_row(canvas, 24.0, y, 600.0, h, selected, seconds);
    crate::ui::text_fit(
        canvas,
        label,
        54.0,
        y + 24.0,
        490.0,
        48.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    canvas.text_right(
        "›",
        584.0,
        y + 20.0,
        58.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
}
