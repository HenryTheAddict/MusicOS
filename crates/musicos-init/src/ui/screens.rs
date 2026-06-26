use crate::app::Focus;
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{
    button, draw_globe, focus_halo, inner_shadow, liquid_glass, rr, shadow_rr, FontWeight,
};

pub fn draw_first(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_back(transition_t);
    let (top, _) = theme.gradient_colors();
    canvas.vertical_gradient(Color::WHITE, top);

    let button_y = 286.0 + (1.0 - t) * 48.0;
    button(
        canvas,
        440.0,
        button_y,
        400.0,
        170.0,
        lang.translate("get_started"),
        theme.primary_color(),
        Color::WHITE,
        seconds,
        focus == Focus::GetStarted,
    );

    let lang_y = 504.0 + (1.0 - t) * 64.0;
    if focus == Focus::Language {
        focus_halo(canvas, 440.0, lang_y, 400.0, 170.0, 20.0, seconds, 0.55);
    }
    liquid_glass(canvas, 440.0, lang_y, 400.0, 170.0, 20.0, 220);
    inner_shadow(canvas, 440.0, lang_y, 400.0, 170.0, 20.0, 4.0);
    draw_globe(canvas, 640.0, lang_y + 84.5, 64.0, 10.0, Color::BLACK);
    if focus == Focus::Language {
        canvas.register_selected(440.0, lang_y, 400.0, 170.0, 20.0, 8.0);
    }

    canvas.text(
        "Build 1",
        3.0,
        900.0 + (1.0 - ease_out_cubic(transition_t)) * 20.0,
        64.0,
        Color::rgba(255, 255, 255, 128),
        FontWeight::Bold,
    );
}

pub fn draw_welcome(
    canvas: &mut Canvas<'_>,
    _seconds: f32,
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    let alpha = (255.0 * t).round() as u8;
    let (top, _) = theme.gradient_colors();
    canvas.vertical_gradient(Color::WHITE, top);
    canvas.text(
        lang.translate("hello"),
        416.0,
        194.0 + (1.0 - t) * 38.0,
        256.0,
        Color::rgba(0, 0, 0, alpha),
        FontWeight::Bold,
    );
    canvas.text(
        lang.translate("welcome"),
        268.0,
        450.0 + (1.0 - t) * 42.0,
        64.0,
        Color::rgba(255, 255, 255, alpha),
        FontWeight::Bold,
    );
    canvas.text_right(
        lang.translate("grab_device"),
        1011.0,
        578.0 + (1.0 - t) * 46.0,
        48.0,
        Color::rgba(0, 0, 0, alpha),
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
    let (top_color, bottom_color) =
        if let (Some(top), Some(bottom)) = (canvas.theme_top_color, canvas.theme_bottom_color) {
            (top, bottom)
        } else {
            active_theme.gradient_colors()
        };
    canvas.vertical_gradient(top_color, bottom_color);

    canvas.text(
        lang.translate("choose_theme"),
        258.0,
        145.0 + (1.0 - t) * 38.0,
        128.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    let panel_y = 309.0 + (1.0 - ease_out_back(transition_t)) * 52.0;
    shadow_rr(canvas, 70.0, panel_y, 1139.0, 300.0, 150.0, 8.0, 16.0);
    liquid_glass(canvas, 70.0, panel_y, 1139.0, 300.0, 150.0, 38);

    let theme_colors = [
        (ThemeName::Blue, 232.0, ThemeName::Blue.primary_color()),
        (ThemeName::Red, 504.0, ThemeName::Red.primary_color()),
        (ThemeName::Green, 776.0, ThemeName::Green.primary_color()),
        (ThemeName::Purple, 1048.0, ThemeName::Purple.primary_color()),
    ];

    for (index, (theme_name, cx, color)) in theme_colors.into_iter().enumerate() {
        let circle_t = ease_out_back(stagger(t, index, 0.05, 0.8));
        let cy = 459.0 + (1.0 - circle_t) * 28.0;
        circle_gradient(canvas, cx, cy, 128.0, Color::WHITE, color);

        if theme_name == active_theme {
            if matches!(focus, Focus::ThemeSelect) {
                focus_halo(
                    canvas,
                    cx - 128.0,
                    cy - 128.0,
                    256.0,
                    256.0,
                    128.0,
                    seconds,
                    0.0,
                );
                canvas.register_selected(cx - 128.0, cy - 128.0, 256.0, 256.0, 128.0, 16.0);
            } else {
                ellipse(canvas, cx, cy, 128.0, Color::BLACK, 6.0);
            }
        }
    }

    let custom_selected = matches!(focus, Focus::Custom);
    let yeah_selected = matches!(focus, Focus::Yeah);

    button(
        canvas,
        237.0,
        645.0 + (1.0 - t) * 34.0,
        400.0,
        170.0,
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

    button(
        canvas,
        659.0,
        645.0 + (1.0 - t) * 34.0,
        400.0,
        170.0,
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
    lang: Language,
    theme: ThemeName,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    let (top, _) = theme.gradient_colors();
    canvas.vertical_gradient(top, Color::WHITE);
    let globe_x = 74.0 - (1.0 - t) * 42.0;
    liquid_glass(canvas, globe_x, 395.0, 400.0, 170.0, 20.0, 220);
    draw_globe(canvas, globe_x + 200.0, 480.0, 64.0, 6.0, Color::BLACK);
    liquid_glass(
        canvas,
        509.0 + (1.0 - t) * 56.0,
        92.0,
        697.0,
        775.0,
        100.0,
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
            110.0,
            false,
            seconds,
            index as f32 * 0.35,
        );
    }

    canvas.text(
        lang.translate("lang_help"),
        10.0,
        878.0,
        56.0,
        Color::rgba(0, 0, 0, 140),
        FontWeight::Bold,
    );
}

pub fn draw_test_complete(
    canvas: &mut Canvas<'_>,
    _seconds: f32,
    lang: Language,
    transition_t: f32,
) {
    let t = ease_out_cubic(transition_t);
    canvas.clear(Color::WHITE);
    canvas.text(
        lang.translate("test_complete"),
        73.0,
        239.0 + (1.0 - t) * 42.0,
        256.0,
        Color::rgba(0, 0, 0, (255.0 * t).round() as u8),
        FontWeight::Bold,
    );
    canvas.text(
        lang.translate("test_desc"),
        162.0,
        593.0 + (1.0 - t) * 40.0,
        64.0,
        Color::rgba(0, 0, 0, (255.0 * t).round() as u8),
        FontWeight::Bold,
    );
}

fn circle_gradient(canvas: &mut Canvas<'_>, cx: f32, cy: f32, r: f32, top: Color, bottom: Color) {
    let res = canvas.resolution();
    canvas.circle_vertical_gradient(
        res.scale_x(cx),
        res.scale_y(cy),
        r * res.scale(),
        top,
        bottom,
    );
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
    let accent = canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229));
    if selected {
        focus_halo(canvas, x, y, w, row_h, 25.0, seconds, phase);
        rr(
            canvas,
            x,
            y,
            w,
            row_h,
            25.0,
            accent.mix(
                Color::WHITE,
                0.08 + crate::ui::transitions::pulse01(seconds, 3.2, phase) * 0.08,
            ),
        );
        canvas.register_selected(x, y, w, row_h, 25.0, 8.0);
    } else {
        liquid_glass(canvas, x, y, w, row_h, 25.0, 92);
    }

    let text_color = if selected {
        Color::WHITE
    } else if blurred {
        Color::rgba(0, 0, 0, 90)
    } else {
        Color::BLACK
    };

    canvas.text_centered(
        label,
        x + w / 2.0,
        y + if size > 100.0 { 10.0 } else { 28.0 },
        size,
        text_color,
        FontWeight::Bold,
    );

    if blurred {
        for i in 0..6 {
            let grow = i as f32 * 7.0;
            rr(
                canvas,
                x - grow,
                y + 18.0 - grow * 0.25,
                w + grow * 2.0,
                120.0 + grow * 0.5,
                25.0 + grow,
                Color::rgba(255, 255, 255, 18),
            );
        }
    }
}
