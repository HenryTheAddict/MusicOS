use crate::app::Focus;
use crate::audio::EqPreset;
use crate::renderer::Canvas;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{figma_list_row, panel_gradient, rr, text_fit_centered, FontWeight};

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
    crate::ui::draw_figma_background(canvas);

    let items = [
        (Focus::SettingsGeneral, "General", 84.0),
        (Focus::SettingsAudio, "Audio", 202.0),
        (Focus::SettingsLooks, "Appearance", 320.0),
        (Focus::SettingsConnectivity, "Library Scan", 438.0),
        (Focus::SettingsUser, "USB Import", 556.0),
        (Focus::SettingsGetUpdate, "Updates", 674.0),
        (Focus::SettingsAbout, "About", 792.0),
    ];

    for (index, (item_focus, label, y)) in items.into_iter().enumerate() {
        let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.78));
        let x = 32.0 - (1.0 - row_t) * 64.0;
        let selected = matches!(
            (focus, item_focus),
            (Focus::SettingsTheme, Focus::SettingsLooks)
                | (Focus::SettingsLanguage, Focus::SettingsGeneral)
        ) || focus == item_focus;
        figma_list_row(canvas, x, y, 1222.0, 104.0, selected, seconds);
        crate::ui::text_fit(
            canvas,
            label,
            x + 28.0,
            y + 18.0,
            1040.0,
            64.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        canvas.text_right(
            "›",
            x + 1180.0,
            y + 21.0,
            58.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Bold,
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
    seconds: f32,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    rr(
        canvas,
        32.0,
        84.0,
        1216.0,
        824.0,
        48.0,
        crate::ui::surface(canvas),
    );
    text_fit_centered(
        canvas,
        "Device preferences",
        640.0,
        105.0,
        1100.0,
        46.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );

    detail_row(
        canvas,
        207.0 + detail_offset(transition_t, 0),
        "Language",
        focus == Focus::SettingsLanguage,
        seconds,
        0.0,
    );
    detail_row(
        canvas,
        326.0 + detail_offset(transition_t, 1),
        "Date And Time",
        focus == Focus::SettingsDateTime,
        seconds,
        0.35,
    );
    detail_row(
        canvas,
        445.0 + detail_offset(transition_t, 2),
        &format!("Brightness {brightness}%"),
        focus == Focus::SettingsBrightness,
        seconds,
        0.7,
    );
    detail_row(
        canvas,
        564.0 + detail_offset(transition_t, 3),
        &format!("Volume {volume}%"),
        focus == Focus::SettingsVolume,
        seconds,
        1.05,
    );
    detail_row(
        canvas,
        683.0 + detail_offset(transition_t, 4),
        &format!("EQ {}", eq_preset.label()),
        focus == Focus::SettingsEq,
        seconds,
        1.4,
    );
    detail_row(
        canvas,
        802.0 + detail_offset(transition_t, 5),
        &format!("Theme {}", theme_label(theme)),
        focus == Focus::SettingsSwitchObject,
        seconds,
        1.75,
    );

    canvas.text_centered(
        battery_text,
        640.0,
        918.0,
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
    theme: ThemeName,
    seconds: f32,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    let (top, bottom) = theme.gradient_colors();
    crate::ui::draw_figma_background(canvas);
    panel_gradient(canvas, 64.0, 52.0, 1152.0, 856.0, 96.0, top, bottom);

    canvas.text_centered(
        "Set Date & Time",
        640.0,
        82.0,
        94.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    canvas.text_centered(
        "Used for history, sorting, and logs",
        640.0,
        184.0,
        40.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );

    small_date_card(
        canvas,
        90.0 + (1.0 - stagger(transition_t, 0, 0.07, 0.82)) * 42.0,
        288.0,
        "Hour",
        &format!("{hour:02}"),
        focus == Focus::DateHour,
        seconds,
        0.0,
    );
    small_date_card(
        canvas,
        355.0 + (1.0 - stagger(transition_t, 1, 0.07, 0.82)) * 42.0,
        288.0,
        "Minute",
        &format!("{minute:02}"),
        focus == Focus::DateMinute,
        seconds,
        0.35,
    );
    small_date_card(
        canvas,
        620.0 + (1.0 - stagger(transition_t, 2, 0.07, 0.82)) * 42.0,
        288.0,
        "Month",
        &format!("{month:02}"),
        focus == Focus::DateMonth,
        seconds,
        0.7,
    );
    small_date_card(
        canvas,
        885.0 + (1.0 - stagger(transition_t, 3, 0.07, 0.82)) * 42.0,
        288.0,
        "Day",
        &format!("{day:02}"),
        focus == Focus::DateDay,
        seconds,
        1.05,
    );
    wide_date_card(
        canvas,
        190.0 + (1.0 - stagger(transition_t, 4, 0.07, 0.82)) * 42.0,
        602.0,
        "Year",
        &year.to_string(),
        focus == Focus::DateYear,
        seconds,
        1.4,
    );

    let selected = focus == Focus::DateDone;
    figma_list_row(canvas, 690.0, 602.0, 400.0, 148.0, selected, seconds);
    text_fit_centered(
        canvas,
        "Done",
        890.0,
        629.0,
        330.0,
        63.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
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
    figma_list_row(canvas, 149.0, y, 982.0, h, selected, seconds);
    crate::ui::text_fit(
        canvas,
        label,
        178.0,
        y + 24.0,
        820.0,
        48.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    canvas.text_right(
        "›",
        1094.0,
        y + 20.0,
        58.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
}

fn small_date_card(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    label: &str,
    value: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    date_card_sized(
        canvas, x, y, 220.0, 234.0, label, value, selected, seconds, phase,
    );
}

fn wide_date_card(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    label: &str,
    value: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    date_card_sized(
        canvas, x, y, 400.0, 148.0, label, value, selected, seconds, phase,
    );
}

fn date_card_sized(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    value: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    let _ = phase;
    figma_list_row(canvas, x, y, w, h, selected, seconds);
    text_fit_centered(
        canvas,
        label,
        x + w * 0.5,
        y + h * 0.12,
        w - 30.0,
        h * 0.20,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
    text_fit_centered(
        canvas,
        value,
        x + w * 0.5,
        y + h * 0.42,
        w - 26.0,
        h * 0.42,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
}
