use crate::app::Focus;
use crate::audio::EqPreset;
use crate::renderer::Canvas;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{compact_row, text_fit, FontWeight};

pub fn draw_settings(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    _seconds: f32,
    _lang: Language,
    _theme: ThemeName,
    transition_t: f32,
) {
    crate::ui::draw_figma_background(canvas);
    for (index, (item, label, description)) in [
        (
            Focus::SettingsGeneral,
            "General",
            "Language, clock and display",
        ),
        (Focus::SettingsAudio, "Audio", "Sound and playback"),
        (Focus::SettingsLooks, "Appearance", "Theme"),
        (
            Focus::SettingsConnectivity,
            "Library Scan",
            "Find new music",
        ),
        (
            Focus::SettingsUser,
            "USB Import",
            "Import from connected storage",
        ),
        (Focus::SettingsGetUpdate, "Updates", "Handheld software"),
        (
            Focus::SettingsAbout,
            "About",
            "Device and library information",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let selected = focus == item
            || matches!(
                (focus, item),
                (Focus::SettingsTheme, Focus::SettingsLooks)
                    | (Focus::SettingsLanguage, Focus::SettingsGeneral)
            );
        detail_row(
            canvas,
            index,
            94.0,
            label,
            description,
            selected,
            transition_t,
        );
    }
}

pub fn draw_settings_detail(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    brightness: u8,
    volume: u8,
    eq_preset: EqPreset,
    theme: ThemeName,
    battery_text: &str,
    _seconds: f32,
    transition_t: f32,
) {
    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(canvas, "Device preferences", battery_text);
    for (index, (item, label, value)) in [
        (Focus::SettingsLanguage, "Language", String::new()),
        (Focus::SettingsDateTime, "Date & Time", String::new()),
        (
            Focus::SettingsBrightness,
            "Brightness",
            format!("{brightness}%"),
        ),
        (Focus::SettingsVolume, "Volume", format!("{volume}%")),
        (Focus::SettingsEq, "Equalizer", eq_preset.label().to_owned()),
        (
            Focus::SettingsSwitchObject,
            "Theme",
            theme_label(theme).to_owned(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        detail_row(
            canvas,
            index,
            225.0,
            label,
            if value.is_empty() { "›" } else { &value },
            focus == item,
            transition_t,
        );
    }
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
    _seconds: f32,
    transition_t: f32,
) {
    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(
        canvas,
        &format!("{hour:02}:{minute:02} · {year:04}-{month:02}-{day:02}"),
        "Left / Right to choose a field · Up / Down to adjust",
    );
    for (index, (item, label, value)) in [
        (Focus::DateHour, "Hour", format!("{hour:02}")),
        (Focus::DateMinute, "Minute", format!("{minute:02}")),
        (Focus::DateMonth, "Month", format!("{month:02}")),
        (Focus::DateDay, "Day", format!("{day:02}")),
        (Focus::DateYear, "Year", format!("{year:04}")),
        (Focus::DateDone, "Done", String::new()),
    ]
    .into_iter()
    .enumerate()
    {
        detail_row(
            canvas,
            index,
            225.0,
            label,
            &value,
            focus == item,
            transition_t,
        );
    }
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
    index: usize,
    top: f32,
    label: &str,
    value: &str,
    selected: bool,
    transition_t: f32,
) {
    let t = ease_out_back(stagger(ease_out_cubic(transition_t), index, 0.05, 0.78));
    let x = 44.0 - (1.0 - t) * 26.0;
    let y = top + index as f32 * 96.0;
    compact_row(canvas, x, y, 1192.0, 84.0, selected);
    text_fit(
        canvas,
        label,
        x + 24.0,
        y + 20.0,
        540.0,
        36.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    let value = super::fitted_text(canvas, value, 576.0, 36.0, FontWeight::Regular);
    canvas.text_right(
        &value,
        x + 1166.0,
        y + 20.0,
        36.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Regular,
    );
}
