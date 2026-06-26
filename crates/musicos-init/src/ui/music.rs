use crate::app::Focus;
use crate::database::{LibraryCounts, Pin, Track};
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{
    animated_focus_blue, figma_list_row, figma_split_panels, focus_halo, liquid_glass,
    panel_gradient, rr, shadow_rr, text_fit_centered, FontWeight,
};

pub fn draw_music_menu(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    pins: &[Pin],
    counts: LibraryCounts,
    queue_len: usize,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    figma_split_panels(canvas, theme, 375.0, 427.0, 832.0);
    canvas.text_centered(
        lang.translate("pins"),
        208.0,
        40.0,
        72.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    if pins.is_empty() {
        let help = lang.translate("pins_help");
        let lines: Vec<&str> = help.lines().collect();
        crate::ui::centered_lines(
            canvas,
            &lines,
            208.0,
            252.0,
            36.0,
            36.0,
            Color::BLACK,
            FontWeight::Regular,
        );
    } else {
        let limit = pins.len().min(5);
        for i in 0..limit {
            let y = 180.0 + i as f32 * 130.0;

            crate::ui::liquid_glass(canvas, 40.0, y, 335.0, 110.0, 30.0, 160);

            canvas.text(
                &pins[i].name,
                60.0,
                y + 16.0,
                40.0,
                Color::BLACK,
                FontWeight::Bold,
            );
            canvas.text(
                &pins[i].subtitle,
                60.0,
                y + 60.0,
                28.0,
                Color::rgb(80, 80, 80),
                FontWeight::Regular,
            );
        }
    }

    let items = [
        (Focus::Albums, lang.translate("albums"), counts.albums, 42.0),
        (Focus::Songs, lang.translate("songs"), counts.tracks, 149.0),
        (
            Focus::Artists,
            lang.translate("artists"),
            counts.artists,
            256.0,
        ),
        (
            Focus::Playlists,
            lang.translate("playlists"),
            counts.playlists,
            363.0,
        ),
        (
            Focus::Favorites,
            lang.translate("favorites"),
            counts.favorites,
            470.0,
        ),
        (
            Focus::Queue,
            lang.translate("queue"),
            queue_len as i64,
            577.0,
        ),
        (Focus::History, lang.translate("history"), 0, 684.0),
        (Focus::ShuffleAll, lang.translate("shuffle_all"), 0, 791.0),
    ];

    for (item_focus, label, badge, y) in items {
        let index = match item_focus {
            Focus::Albums => 0,
            Focus::Songs => 1,
            Focus::Artists => 2,
            Focus::Playlists => 3,
            Focus::Favorites => 4,
            Focus::Queue => 5,
            Focus::History => 6,
            Focus::ShuffleAll => 7,
            _ => 0,
        };
        let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.76));
        let x = 487.0 + (1.0 - row_t) * 70.0;
        let selected = focus == item_focus;
        figma_list_row(canvas, x, y, 711.0, 92.0, selected, seconds);
        text_fit_centered(
            canvas,
            label,
            x + 355.0,
            y + 22.0,
            620.0,
            48.0,
            Color::BLACK,
            FontWeight::Regular,
        );
        if badge > 0 {
            rr(canvas, x + 590.0, y + 25.0, 74.0, 42.0, 21.0, Color::BLACK);
            canvas.text_centered(
                &badge.to_string(),
                x + 627.0,
                y + 30.0,
                28.0,
                Color::WHITE,
                FontWeight::Bold,
            );
        }
    }
}

pub fn draw_items_list(
    canvas: &mut Canvas<'_>,
    title: &str,
    items: &[String],
    selected_idx: usize,
    theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    draw_collection_shell(canvas, title, theme);

    if items.is_empty() {
        return;
    }

    for (idx, item) in items.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 248.0 + idx as f32 * 144.0 - scroll_offset;

        if y + 128.0 < 252.0 || y > 850.0 {
            continue;
        }

        collection_row(
            canvas,
            y,
            item,
            "Local library",
            selected,
            idx,
            transition_t,
            seconds,
        );
    }
    draw_alpha_index(
        canvas,
        &items.iter().map(|item| item.as_str()).collect::<Vec<_>>(),
    );
    draw_collection_masks(canvas);
}

pub fn draw_tracks_list(
    canvas: &mut Canvas<'_>,
    title: &str,
    tracks: &[Track],
    selected_idx: usize,
    theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    draw_collection_shell(canvas, title, theme);

    if tracks.is_empty() {
        return;
    }

    for (idx, track) in tracks.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 248.0 + idx as f32 * 144.0 - scroll_offset;

        if y + 128.0 < 252.0 || y > 850.0 {
            continue;
        }

        collection_row(
            canvas,
            y,
            &track.title,
            &track.artist,
            selected,
            idx,
            transition_t,
            seconds,
        );
    }
    draw_alpha_index(
        canvas,
        &tracks
            .iter()
            .map(|track| track.title.as_str())
            .collect::<Vec<_>>(),
    );
    draw_collection_masks(canvas);
}

fn draw_alpha_index(canvas: &mut Canvas<'_>, labels: &[&str]) {
    if labels.len() < 12 {
        return;
    }
    let mut letters = Vec::new();
    for label in labels {
        let letter = label
            .chars()
            .find(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('#');
        if letters.last().copied() != Some(letter) {
            letters.push(letter);
        }
    }
    if letters.len() < 3 {
        return;
    }
    let step = (560.0 / letters.len().max(1) as f32).clamp(24.0, 42.0);
    let mut y = 276.0;
    for letter in letters.into_iter().take(18) {
        canvas.text_centered(
            &letter.to_string(),
            1152.0,
            y,
            22.0,
            Color::rgba(0, 0, 0, 160),
            FontWeight::Bold,
        );
        y += step;
    }
}

fn draw_collection_shell(canvas: &mut Canvas<'_>, title: &str, theme: ThemeName) {
    crate::ui::draw_figma_background(canvas);
    let (top_color, bottom_color) =
        if let (Some(top), Some(bottom)) = (canvas.theme_top_color, canvas.theme_bottom_color) {
            (top, bottom)
        } else {
            theme.gradient_colors()
        };

    panel_gradient(
        canvas,
        49.0,
        32.0,
        1210.0,
        896.0,
        128.0,
        top_color,
        bottom_color,
    );
    rr(
        canvas,
        106.0,
        82.0,
        1095.0,
        132.0,
        66.0,
        Color::rgba(255, 255, 255, 64),
    );
    text_fit_centered(
        canvas,
        title,
        653.5,
        100.0,
        960.0,
        96.0,
        Color::BLACK,
        FontWeight::Regular,
    );

    shadow_rr(canvas, 106.0, 232.0, 1095.0, 650.0, 169.0, 10.0, 16.0);
    rr(canvas, 106.0, 232.0, 1095.0, 650.0, 169.0, Color::WHITE);
}

fn draw_collection_masks(canvas: &mut Canvas<'_>) {
    rr(canvas, 106.0, 232.0, 1095.0, 28.0, 14.0, Color::WHITE);
    rr(canvas, 106.0, 846.0, 1095.0, 36.0, 18.0, Color::WHITE);
}

fn collection_row(
    canvas: &mut Canvas<'_>,
    y: f32,
    title: &str,
    subtitle: &str,
    selected: bool,
    index: usize,
    transition_t: f32,
    seconds: f32,
) {
    let x = 207.0;
    let w = 893.0;
    let h = 128.0;
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
    let y = y + (1.0 - row_t) * 46.0;
    let fill = if selected {
        animated_focus_blue(seconds, index as f32 * 0.4)
    } else {
        Color::rgba(0, 0, 0, 64)
    };
    if selected {
        focus_halo(canvas, x, y, w, h, 105.0, seconds, index as f32 * 0.4);
    }
    rr(canvas, x, y, w, h, 105.0, fill);
    liquid_glass(canvas, x, y, w, h, 105.0, if selected { 24 } else { 22 });
    rr(canvas, x, y, 128.0, h, 64.0, Color::BLACK);

    let text_color = if selected { Color::WHITE } else { Color::BLACK };
    if selected {
        if canvas.text_width(title, 64.0, FontWeight::Regular) > 680.0 {
            let (top, bottom) = wrap_two_lines(canvas, title, 660.0, 48.0)
                .unwrap_or_else(|| (title.to_string(), String::new()));
            canvas.text_centered(
                &top,
                x + 128.0 + 351.0,
                y + 22.0,
                48.0,
                text_color,
                FontWeight::Regular,
            );
            if !bottom.is_empty() {
                canvas.text_centered(
                    &bottom,
                    x + 128.0 + 351.0,
                    y + 66.0,
                    48.0,
                    text_color,
                    FontWeight::Regular,
                );
            }
            return;
        }
    }

    text_fit_centered(
        canvas,
        title,
        x + 128.0 + 351.0,
        y + if subtitle.is_empty() { 32.0 } else { 18.0 },
        680.0,
        64.0,
        text_color,
        FontWeight::Regular,
    );
    if !subtitle.is_empty() {
        text_fit_centered(
            canvas,
            subtitle,
            x + 128.0 + 351.0,
            y + 75.0,
            500.0,
            32.0,
            text_color,
            FontWeight::Regular,
        );
    }
}

fn wrap_two_lines(
    canvas: &Canvas<'_>,
    title: &str,
    max_w: f32,
    size: f32,
) -> Option<(String, String)> {
    let words: Vec<&str> = title.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }

    let mut best: Option<(String, String)> = None;
    for split in 1..words.len() {
        if words.len() - split == 1 && words.len() > 2 {
            continue;
        }
        let top = words[..split].join(" ");
        let bottom = words[split..].join(" ");
        let top_w = canvas.text_width(&top, size, FontWeight::Regular);
        let bottom_w = canvas.text_width(&bottom, size, FontWeight::Regular);
        if top_w > max_w || bottom_w > max_w {
            continue;
        }
        best = Some((top, bottom));
    }

    best
}
