use crate::app::Focus;
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{button, button_with_radius, draw_globe, liquid_glass, rr, FontWeight};

pub fn draw_first(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    rr(
        canvas,
        56.0,
        110.0,
        1168.0,
        774.0,
        56.0,
        crate::ui::surface(canvas),
    );
    canvas.text_centered(
        "musicOS",
        640.0,
        224.0 + (1.0 - t) * 24.0,
        128.0,
        crate::ui::INK,
        FontWeight::Bold,
    );
    crate::ui::text_fit_centered(
        canvas,
        lang.translate("first_tagline"),
        640.0,
        386.0,
        1000.0,
        38.0,
        crate::ui::INK_SECONDARY,
        FontWeight::Regular,
    );

    button(
        canvas,
        420.0,
        504.0,
        440.0,
        110.0,
        lang.translate("get_started"),
        theme.primary_color(),
        Color::WHITE,
        seconds,
        focus == Focus::GetStarted,
    );
    button(
        canvas,
        420.0,
        642.0,
        440.0,
        100.0,
        lang.translate("language_label"),
        theme.primary_color(),
        Color::WHITE,
        seconds,
        focus == Focus::Language,
    );
}

pub fn draw_welcome(
    canvas: &mut Canvas<'_>,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let offset = (1.0 - ease_out_cubic(transition_t)) * 22.0;
    crate::ui::draw_figma_background(canvas);
    rr(
        canvas,
        74.0,
        102.0,
        1132.0,
        800.0,
        48.0,
        crate::ui::surface(canvas),
    );
    crate::ui::text_fit(
        canvas,
        lang.translate("controls_title"),
        138.0,
        144.0 + offset,
        1004.0,
        70.0,
        crate::ui::INK,
        FontWeight::Bold,
    );
    crate::ui::text_fit(
        canvas,
        lang.translate("controls_hint"),
        138.0,
        235.0 + offset,
        1004.0,
        34.0,
        crate::ui::INK_SECONDARY,
        FontWeight::Regular,
    );
    for (index, (key, label)) in [
        ("D-pad", "controls_move"),
        ("A", "controls_select"),
        ("B", "controls_back"),
        ("X / R1", "controls_more"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 308.0 + index as f32 * 106.0;
        rr(
            canvas,
            138.0,
            y,
            1004.0,
            88.0,
            26.0,
            crate::ui::control(canvas),
        );
        rr(
            canvas,
            154.0,
            y + 12.0,
            180.0,
            64.0,
            32.0,
            crate::ui::surface(canvas),
        );
        canvas.text_centered(key, 244.0, y + 25.0, 33.0, crate::ui::INK, FontWeight::Bold);
        crate::ui::text_fit(
            canvas,
            lang.translate(label),
            366.0,
            y + 24.0,
            744.0,
            36.0,
            crate::ui::INK,
            FontWeight::Regular,
        );
    }
    button(
        canvas,
        746.0,
        756.0,
        396.0,
        100.0,
        lang.translate("controls_continue"),
        theme.primary_color(),
        Color::WHITE,
        seconds,
        true,
    );
}

pub fn draw_theme_selection(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    active_theme: ThemeName,
    seconds: f32,
    lang: Language,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);

    crate::ui::text_fit_centered(
        canvas,
        lang.translate("choose_theme"),
        640.0,
        142.0 + (1.0 - t) * 38.0,
        1120.0,
        64.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    let panel_y = 309.0 + (1.0 - ease_out_back(transition_t)) * 52.0;
    rr(
        canvas,
        70.0,
        panel_y,
        1139.0,
        300.0,
        48.0,
        crate::ui::surface(canvas),
    );

    let theme_colors = [
        (ThemeName::Blue, 232.0, ThemeName::Blue.primary_color()),
        (ThemeName::Red, 504.0, ThemeName::Red.primary_color()),
        (ThemeName::Green, 776.0, ThemeName::Green.primary_color()),
        (ThemeName::Purple, 1048.0, ThemeName::Purple.primary_color()),
    ];

    for (index, (theme_name, cx, color)) in theme_colors.into_iter().enumerate() {
        let circle_t = ease_out_back(stagger(t, index, 0.05, 0.8));
        let cy = 459.0 + (1.0 - circle_t) * 28.0;
        rr(canvas, cx - 100.0, cy - 100.0, 200.0, 200.0, 100.0, color);

        if theme_name == active_theme {
            if matches!(focus, Focus::ThemeSelect) {
                crate::ui::register_focus(canvas, cx - 108.0, cy - 108.0, 216.0, 216.0, 108.0);
            } else {
                ellipse(canvas, cx, cy, 108.0, crate::ui::INK, 6.0);
            }
        }
    }

    let custom_selected = matches!(focus, Focus::Custom);
    let yeah_selected = matches!(focus, Focus::Yeah);

    button_with_radius(
        canvas,
        237.0,
        645.0 + (1.0 - t) * 34.0,
        400.0,
        170.0,
        32.0,
        lang.translate("custom"),
        if custom_selected {
            Color::rgba(255, 255, 255, 230)
        } else {
            Color::WHITE
        },
        Color::BLACK,
        seconds,
        custom_selected,
    );

    button_with_radius(
        canvas,
        659.0,
        645.0 + (1.0 - t) * 34.0,
        400.0,
        170.0,
        32.0,
        lang.translate("yeah"),
        if yeah_selected {
            active_theme.primary_color()
        } else {
            Color::WHITE
        },
        if yeah_selected {
            Color::WHITE
        } else {
            Color::BLACK
        },
        seconds,
        yeah_selected,
    );
}

pub fn draw_language_selection(
    canvas: &mut Canvas<'_>,
    _active_lang: Language,
    selected_idx: usize,
    scroll_offset: f32,
    seconds: f32,
    _lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    let _ = theme;
    crate::ui::draw_figma_background(canvas);
    let globe_x = 74.0 - (1.0 - t) * 42.0;
    liquid_glass(canvas, globe_x, 395.0, 400.0, 170.0, 20.0, 220);
    draw_globe(canvas, globe_x + 200.0, 480.0, 64.0, 6.0, Color::BLACK);
    liquid_glass(
        canvas,
        509.0 + (1.0 - t) * 56.0,
        92.0,
        697.0,
        775.0,
        32.0,
        220,
    );

    let items = [
        (Language::En, "English"),
        (Language::Es, "Español"),
        (Language::De, "Deutsch"),
        (Language::Ja, "日本語"),
        (Language::Zh, "中文"),
    ];

    for (index, (_item_lang, label)) in items.into_iter().enumerate() {
        let selected = index == selected_idx;
        let y = 112.0 + index as f32 * 144.0 - scroll_offset;
        if y + 124.0 < 92.0 || y > 867.0 {
            continue;
        }
        let row_t = ease_out_back(stagger(t, index, 0.05, 0.75));
        let x = 529.0 + (1.0 - row_t) * 70.0;
        let w = 657.0;
        lang_row(
            canvas,
            x,
            y,
            w,
            label,
            selected,
            57.0,
            false,
            seconds,
            index as f32 * 0.35,
        );
    }
}

pub fn draw_test_complete(
    canvas: &mut Canvas<'_>,
    _seconds: f32,
    lang: Language,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    rr(
        canvas,
        74.0,
        152.0,
        1132.0,
        640.0,
        48.0,
        crate::ui::surface(canvas),
    );
    crate::ui::text_fit_centered(
        canvas,
        lang.translate("test_complete"),
        640.0,
        294.0 + (1.0 - t) * 28.0,
        1010.0,
        72.0,
        crate::ui::INK,
        FontWeight::Bold,
    );
    for (index, line) in lang.translate("test_desc").lines().enumerate() {
        crate::ui::text_fit_centered(
            canvas,
            line,
            640.0,
            442.0 + index as f32 * 61.0,
            1010.0,
            38.0,
            crate::ui::INK_SECONDARY,
            FontWeight::Regular,
        );
    }
}

fn ellipse(canvas: &mut Canvas<'_>, cx: f32, cy: f32, r: f32, color: Color, thickness: f32) {
    let res = canvas.resolution();
    canvas.circle_stroke(
        res.scale_x(cx),
        res.scale_y(cy),
        r * res.scale(),
        thickness * res.scale(),
        color,
    );
}

fn lang_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    selected: bool,
    size: f32,
    blurred: bool,
    seconds: f32,
    phase: f32,
) {
    let row_h = 124.0;
    let _ = (blurred, phase);
    crate::ui::figma_list_row(canvas, x, y, w, row_h, selected, seconds);
    let text_color = if selected {
        Color::WHITE
    } else {
        crate::ui::INK
    };

    canvas.text_centered(
        label,
        x + w / 2.0,
        y + 30.0,
        size,
        text_color,
        FontWeight::Bold,
    );
}
