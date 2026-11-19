use crate::app::Focus;
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{button, button_with_radius, draw_globe, rr, FontWeight};

pub fn draw_first(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    crate::ui::rows_preview(canvas);
    canvas.text_centered(
        "musicOS",
        954.0,
        224.0 + (1.0 - t) * 24.0,
        110.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    crate::ui::text_fit_centered(
        canvas,
        lang.translate("first_tagline"),
        954.0,
        386.0,
        532.0,
        38.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Regular,
    );

    button(
        canvas,
        24.0,
        240.0,
        600.0,
        110.0,
        lang.translate("get_started"),
        theme.primary_color(),
        Color::WHITE,
        seconds,
        focus == Focus::GetStarted,
    );
    button(
        canvas,
        24.0,
        368.0,
        600.0,
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
    crate::ui::rows_preview(canvas);
    crate::ui::text_fit(
        canvas,
        lang.translate("controls_title"),
        686.0,
        142.0 + offset,
        540.0,
        53.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    crate::ui::text_fit(
        canvas,
        lang.translate("controls_hint"),
        686.0,
        226.0 + offset,
        540.0,
        34.0,
        crate::ui::ink_secondary(canvas),
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
        let y = 130.0 + index as f32 * 106.0;
        rr(
            canvas,
            24.0,
            y,
            600.0,
            88.0,
            26.0,
            crate::ui::control(canvas),
        );
        rr(
            canvas,
            38.0,
            y + 12.0,
            133.0,
            64.0,
            32.0,
            crate::ui::surface(canvas),
        );
        canvas.text_centered(
            key,
            104.5,
            y + 25.0,
            33.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        crate::ui::text_fit(
            canvas,
            lang.translate(label),
            188.0,
            y + 24.0,
            414.0,
            36.0,
            crate::ui::ink(canvas),
            FontWeight::Regular,
        );
    }
    button(
        canvas,
        24.0,
        592.0,
        600.0,
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
    crate::ui::rows_preview(canvas);
    crate::ui::preview_heading(
        canvas,
        lang.translate("choose_theme"),
        theme_name(active_theme),
    );
    rr(
        canvas,
        685.0,
        250.0,
        538.0,
        538.0,
        46.0,
        crate::ui::control(canvas),
    );
    rr(
        canvas,
        735.0,
        300.0,
        438.0,
        94.0,
        47.0,
        crate::ui::selected_surface(canvas),
    );
    rr(
        canvas,
        751.0,
        316.0,
        62.0,
        62.0,
        31.0,
        crate::ui::accent(canvas),
    );

    for (index, theme_name) in ThemeName::OPTIONS.into_iter().enumerate() {
        let cx = 126.0 + (index % 3) as f32 * 188.0;
        let color = theme_name.primary_color();
        let circle_t = ease_out_back(stagger(t, index, 0.05, 0.8));
        let cy = 255.0 + (index / 3) as f32 * 172.0 + (1.0 - circle_t) * 28.0;
        rr(canvas, cx - 60.0, cy - 60.0, 120.0, 120.0, 60.0, color);

        if theme_name == active_theme {
            if matches!(focus, Focus::ThemeSelect) {
                crate::ui::register_focus(canvas, cx - 66.0, cy - 66.0, 132.0, 132.0, 66.0);
            } else {
                ellipse(canvas, cx, cy, 66.0, crate::ui::ink(canvas), 6.0);
            }
        }
    }

    let custom_selected = matches!(focus, Focus::Custom);
    let yeah_selected = matches!(focus, Focus::Yeah);

    button_with_radius(
        canvas,
        24.0,
        582.0 + (1.0 - t) * 25.0,
        288.0,
        105.0,
        32.0,
        lang.translate("custom"),
        if custom_selected {
            Color::rgba(255, 255, 255, 230)
        } else {
            Color::WHITE
        },
        crate::ui::ink(canvas),
        seconds,
        custom_selected,
    );

    button_with_radius(
        canvas,
        336.0,
        582.0 + (1.0 - t) * 25.0,
        288.0,
        105.0,
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
            crate::ui::ink(canvas)
        },
        seconds,
        yeah_selected,
    );
}

fn theme_name(theme: ThemeName) -> &'static str {
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
    crate::ui::rows_preview(canvas);
    draw_globe(canvas, 954.0, 330.0, 108.0, 8.0, crate::ui::accent(canvas));

    let items = [
        (Language::En, "English"),
        (Language::Es, "Español"),
        (Language::De, "Deutsch"),
        (Language::Ja, "日本語"),
        (Language::Zh, "中文"),
    ];

    if let Some((_, label)) = items.get(selected_idx) {
        crate::ui::preview_heading(canvas, label, "Language");
    }
    for (index, (_item_lang, label)) in items.into_iter().enumerate() {
        let selected = index == selected_idx;
        let y = 112.0 + index as f32 * 144.0 - scroll_offset;
        if y + 124.0 < 92.0 || y > 867.0 {
            continue;
        }
        let row_t = ease_out_back(stagger(t, index, 0.05, 0.75));
        let x = 24.0 - (1.0 - row_t) * 35.0;
        let w = 600.0;
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
    crate::ui::rows_preview(canvas);
    crate::ui::preview_heading(canvas, lang.translate("test_complete"), "");
    for (index, line) in lang.translate("test_desc").lines().enumerate() {
        crate::ui::text_fit_centered(
            canvas,
            line,
            324.0,
            260.0 + index as f32 * 94.0 + (1.0 - t) * 28.0,
            578.0,
            38.0,
            crate::ui::ink_secondary(canvas),
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
    let text_color = crate::ui::ink(canvas);

    canvas.text_centered(
        label,
        x + w / 2.0,
        y + 30.0,
        size,
        text_color,
        FontWeight::Bold,
    );
}
