use crate::app::Focus;
use crate::renderer::Canvas;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{button_with_radius, compact_row, rr, FontWeight};

pub fn draw_first(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    _seconds: f32,
    lang: Language,
    _theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(canvas, "musicOS", lang.translate("first_tagline"));
    for (index, (item, label)) in [
        (Focus::GetStarted, "get_started"),
        (Focus::Language, "language_label"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 252.0 + index as f32 * 112.0 + (1.0 - t) * 24.0;
        compact_row(canvas, 44.0, y, 1192.0, 96.0, focus == item);
        crate::ui::text_fit(
            canvas,
            lang.translate(label),
            68.0,
            y + 22.0,
            1110.0,
            40.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
    }
}

pub fn draw_welcome(
    canvas: &mut Canvas<'_>,
    _seconds: f32,
    lang: Language,
    _theme: ThemeName,
    transition_t: f32,
) {
    let offset = (1.0 - ease_out_cubic(transition_t)) * 22.0;
    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(
        canvas,
        lang.translate("controls_title"),
        lang.translate("controls_hint"),
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
        let y = 230.0 + index as f32 * 102.0 + offset;
        compact_row(canvas, 44.0, y, 1192.0, 88.0, false);
        crate::ui::text_fit(
            canvas,
            key,
            68.0,
            y + 22.0,
            155.0,
            34.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        crate::ui::text_fit(
            canvas,
            lang.translate(label),
            252.0,
            y + 22.0,
            940.0,
            34.0,
            crate::ui::ink(canvas),
            FontWeight::Regular,
        );
    }
    let instruction = lang.translate("controls_continue");
    crate::ui::text_fit(
        canvas,
        instruction,
        68.0,
        694.0 + offset,
        1110.0,
        35.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
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
    crate::ui::page_heading(
        canvas,
        lang.translate("choose_theme"),
        theme_label(active_theme),
    );
    for (index, theme_name) in ThemeName::OPTIONS.into_iter().enumerate() {
        let cx = 145.0 + index as f32 * 198.0;
        let color = theme_name.primary_color();
        let circle_t = ease_out_back(stagger(t, index, 0.05, 0.8));
        let cy = 334.0 + (1.0 - circle_t) * 28.0;
        rr(canvas, cx - 54.0, cy - 54.0, 108.0, 108.0, 16.0, color);

        if theme_name == active_theme {
            if matches!(focus, Focus::ThemeSelect) {
                crate::ui::register_focus(canvas, cx - 54.0, cy - 54.0, 108.0, 108.0, 16.0);
            } else {
                crate::ui::text_fit_centered(
                    canvas,
                    theme_label(theme_name),
                    cx,
                    cy + 72.0,
                    170.0,
                    27.0,
                    crate::ui::ink_secondary(canvas),
                    FontWeight::Bold,
                );
            }
        }
    }

    let custom_selected = matches!(focus, Focus::Custom);
    let yeah_selected = matches!(focus, Focus::Yeah);

    button_with_radius(
        canvas,
        176.0,
        582.0 + (1.0 - t) * 25.0,
        448.0,
        90.0,
        14.0,
        lang.translate("custom"),
        crate::ui::control(canvas),
        crate::ui::ink(canvas),
        seconds,
        custom_selected,
    );

    button_with_radius(
        canvas,
        656.0,
        582.0 + (1.0 - t) * 25.0,
        448.0,
        90.0,
        14.0,
        lang.translate("yeah"),
        crate::ui::control(canvas),
        crate::ui::ink(canvas),
        seconds,
        yeah_selected,
    );
}

fn theme_label(theme: ThemeName) -> &'static str {
    match theme {
        ThemeName::Blue => "Blue",
        ThemeName::Seaglass => "Seaglass",
        ThemeName::Red => "Red",
        ThemeName::Green => "Green",
        ThemeName::Purple => "Purple",
        ThemeName::Dark => "Dark",
    }
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
        let x = 44.0 - (1.0 - row_t) * 26.0;
        let w = 1192.0;
        lang_row(
            canvas,
            x,
            y,
            w,
            label,
            selected,
            40.0,
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
    crate::ui::page_heading(canvas, lang.translate("test_complete"), "");
    for (index, line) in lang.translate("test_desc").lines().enumerate() {
        crate::ui::text_fit_centered(
            canvas,
            line,
            640.0,
            260.0 + index as f32 * 94.0 + (1.0 - t) * 28.0,
            1140.0,
            38.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Regular,
        );
    }
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
    let _ = (blurred, phase, seconds);
    compact_row(canvas, x, y, w, row_h, selected);
    let text_color = crate::ui::ink(canvas);

    canvas.text(
        label,
        x + 24.0,
        y + 30.0,
        size,
        text_color,
        FontWeight::Bold,
    );
}
