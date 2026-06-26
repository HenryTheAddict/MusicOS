use crate::app::Focus;
use crate::audio::EqPreset;
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, pulse01, stagger};
use crate::ui::{
    focus_halo, liquid_glass, panel_gradient, rr, rr_stroke, text_fit_centered, FontWeight,
};

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
    let (theme_top, theme_bottom) = theme.gradient_colors();
    let theme_focus = theme_top.mix(theme_bottom, 0.35);
    crate::ui::draw_figma_background(canvas);

    let items = [
        (Focus::SettingsGeneral, "General", 32.0, 22.0),
        (Focus::SettingsAudio, "Audio", 32.0, 151.0),
        (Focus::SettingsLooks, "Looks", 32.0, 280.0),
        (Focus::SettingsAbout, "About", 32.0, 409.0),
        (Focus::SettingsConnectivity, "Connectivity", 26.0, 538.0),
        (Focus::SettingsGetUpdate, "Get Update", 32.0, 667.0),
        (Focus::SettingsUser, "User", 32.0, 796.0),
    ];

    for (index, (item_focus, label, x, y)) in items.into_iter().enumerate() {
        let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.78));
        let x = x - (1.0 - row_t) * 64.0;
        let selected = matches!(
            (focus, item_focus),
            (Focus::SettingsTheme, Focus::SettingsLooks)
                | (Focus::SettingsLanguage, Focus::SettingsGeneral)
        ) || focus == item_focus;
        if selected {
            focus_halo(
                canvas,
                x,
                y,
                1222.0,
                135.0,
                67.5,
                seconds,
                index as f32 * 0.32,
            );
        }
        rr(
            canvas,
            x,
            y,
            1222.0,
            135.0,
            67.5,
            if selected {
                let pulse = pulse01(seconds, 3.2, index as f32 * 0.32);
                theme_focus.mix(theme_top.mix(Color::WHITE, pulse * 0.14), 0.34)
            } else {
                Color::rgb(244, 246, 248)
            },
        );
        liquid_glass(
            canvas,
            x,
            y,
            1222.0,
            135.0,
            67.5,
            if selected { 22 } else { 36 },
        );
        text_fit_centered(
            canvas,
            label,
            x + 611.0,
            y + 19.5,
            1120.0,
            96.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Regular,
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
    let (theme_top, theme_bottom) = theme.gradient_colors();

    rr(canvas, 29.0, 32.0, 1222.0, 135.0, 67.5, theme_top);
    liquid_glass(canvas, 29.0, 32.0, 1222.0, 135.0, 67.5, 18);
    text_fit_centered(
        canvas,
        "Settings",
        640.0,
        51.5,
        1120.0,
        96.0,
        Color::WHITE,
        FontWeight::Regular,
    );

    rr(
        canvas,
        29.0,
        183.0,
        1222.0,
        746.0,
        128.0,
        theme_bottom.with_alpha(190),
    );
    liquid_glass(canvas, 29.0, 183.0, 1222.0, 746.0, 128.0, 26);

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
    detail_row_left(
        canvas,
        802.0 + detail_offset(transition_t, 5),
        &format!("Theme {}", theme_label(theme)),
        focus == Focus::SettingsSwitchObject,
        seconds,
        1.75,
    );

    let res = canvas.resolution();
    let toggle_pulse = pulse01(seconds, 4.2, 1.1);
    canvas.text_centered(
        battery_text,
        640.0,
        918.0,
        26.0,
        Color::rgba(0, 0, 0, 180),
        FontWeight::Bold,
    );
    canvas.circle(
        res.scale_x(1064.0),
        res.scale_y(826.0),
        (31.0 + toggle_pulse * 3.0) * res.scale(),
        Color::rgb(38, 142, 0).mix(Color::rgb(90, 210, 55), toggle_pulse * 0.25),
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
    liquid_glass(canvas, 64.0, 52.0, 1152.0, 856.0, 96.0, 32);

    canvas.text_centered(
        "Set Date & Time",
        640.0,
        82.0,
        94.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        "Used for history, sorting, and logs",
        640.0,
        184.0,
        40.0,
        Color::rgba(0, 0, 0, 180),
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
    if selected {
        focus_halo(canvas, 690.0, 602.0, 400.0, 148.0, 74.0, seconds, 1.75);
    }
    rr(
        canvas,
        690.0,
        602.0,
        400.0,
        148.0,
        74.0,
        if selected {
            canvas
                .theme_top_color
                .unwrap_or(Color::rgb(63, 143, 229))
                .mix(Color::WHITE, pulse01(seconds, 3.2, 1.75) * 0.14)
        } else {
            Color::WHITE
        },
    );
    liquid_glass(canvas, 690.0, 602.0, 400.0, 148.0, 74.0, 26);
    if selected {
        canvas.register_selected(690.0, 602.0, 400.0, 148.0, 74.0, 7.0);
    }
    text_fit_centered(
        canvas,
        "Done",
        890.0,
        629.0,
        330.0,
        88.0,
        if selected { Color::WHITE } else { Color::BLACK },
        FontWeight::Bold,
    );
}

fn detail_offset(transition_t: f32, index: usize) -> f32 {
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.82));
    (1.0 - row_t) * 42.0
}

fn theme_label(theme: ThemeName) -> &'static str {
    match theme {
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
    let h = 108.0;
    let r = 54.0;
    let accent = canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229));
    if selected {
        focus_halo(canvas, 149.0, y, 982.0, h, r, seconds, phase);
    }
    rr(
        canvas,
        149.0,
        y,
        982.0,
        h,
        r,
        if selected {
            accent.with_alpha(52)
        } else {
            Color::rgba(0, 0, 0, 26)
        },
    );
    liquid_glass(
        canvas,
        149.0,
        y,
        982.0,
        h,
        r,
        if selected { 28 } else { 34 },
    );
    if selected {
        rr_stroke(canvas, 149.0, y, 982.0, h, r, 3.0, accent.with_alpha(120));
    }
    text_fit_centered(
        canvas,
        label,
        640.0,
        y + 16.0,
        860.0,
        76.0,
        Color::BLACK,
        FontWeight::Regular,
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
    let radius = (h * 0.25).min(58.0);
    if selected {
        focus_halo(canvas, x, y, w, h, radius, seconds, phase);
    }
    rr(
        canvas,
        x,
        y,
        w,
        h,
        radius,
        if selected {
            canvas
                .theme_top_color
                .unwrap_or(Color::rgb(63, 143, 229))
                .mix(Color::WHITE, pulse01(seconds, 3.2, phase) * 0.14)
        } else {
            Color::WHITE
        },
    );
    liquid_glass(canvas, x, y, w, h, radius, if selected { 20 } else { 34 });
    if selected {
        canvas.register_selected(x, y, w, h, radius, 7.0);
    }
    text_fit_centered(
        canvas,
        label,
        x + w * 0.5,
        y + h * 0.12,
        w - 30.0,
        h * 0.26,
        Color::BLACK,
        FontWeight::Bold,
    );
    text_fit_centered(
        canvas,
        value,
        x + w * 0.5,
        y + h * 0.42,
        w - 26.0,
        h * 0.42,
        if selected { Color::WHITE } else { Color::BLACK },
        FontWeight::Bold,
    );
}

fn detail_row_left(
    canvas: &mut Canvas<'_>,
    y: f32,
    label: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    let h = 108.0;
    let r = 54.0;
    let accent = canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229));
    if selected {
        focus_halo(canvas, 149.0, y, 982.0, h, r, seconds, phase);
    }
    rr(
        canvas,
        149.0,
        y,
        982.0,
        h,
        r,
        if selected {
            accent.with_alpha(52)
        } else {
            Color::rgba(0, 0, 0, 26)
        },
    );
    liquid_glass(
        canvas,
        149.0,
        y,
        982.0,
        h,
        r,
        if selected { 28 } else { 34 },
    );
    if selected {
        rr_stroke(canvas, 149.0, y, 982.0, h, r, 3.0, accent.with_alpha(120));
    }
    crate::ui::text_fit(
        canvas,
        label,
        188.0,
        y + 16.0,
        840.0,
        76.0,
        Color::BLACK,
        FontWeight::Regular,
    );
}
