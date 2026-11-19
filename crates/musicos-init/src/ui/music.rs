use crate::app::Focus;
use crate::database::{LibraryCounts, Pin, Track};
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{
    figma_list_row, figma_split_panels, liquid_glass, panel_gradient, rr, text_fit_centered,
    FontWeight,
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
        82.0,
        50.0,
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

            crate::ui::text_fit(
                canvas,
                &pins[i].name,
                60.0,
                y + 16.0,
                292.0,
                36.0,
                Color::BLACK,
                FontWeight::Bold,
            );
            crate::ui::text_fit(
                canvas,
                &pins[i].subtitle,
                60.0,
                y + 60.0,
                292.0,
                28.0,
                Color::rgb(80, 80, 80),
                FontWeight::Regular,
            );
        }
    }

    let items = [
        (Focus::Search, lang.translate("search"), 0, 78.0),
        (
            Focus::Albums,
            lang.translate("albums"),
            counts.albums,
            172.0,
        ),
        (Focus::Songs, lang.translate("songs"), counts.tracks, 266.0),
        (
            Focus::Artists,
            lang.translate("artists"),
            counts.artists,
            360.0,
        ),
        (
            Focus::Playlists,
            lang.translate("playlists"),
            counts.playlists,
            454.0,
        ),
        (
            Focus::Favorites,
            lang.translate("favorites"),
            counts.favorites,
            548.0,
        ),
        (
            Focus::Queue,
            lang.translate("queue"),
            queue_len as i64,
            642.0,
        ),
        (Focus::History, lang.translate("history"), 0, 736.0),
        (Focus::ShuffleAll, lang.translate("shuffle_all"), 0, 830.0),
    ];

    for (item_focus, label, badge, y) in items {
        let index = match item_focus {
            Focus::Search => 0,
            Focus::Albums => 1,
            Focus::Songs => 2,
            Focus::Artists => 3,
            Focus::Playlists => 4,
            Focus::Favorites => 5,
            Focus::Queue => 6,
            Focus::History => 7,
            Focus::ShuffleAll => 8,
            _ => 0,
        };
        let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.76));
        let x = 487.0 + (1.0 - row_t) * 70.0;
        let selected = focus == item_focus;
        figma_list_row(canvas, x, y, 711.0, 80.0, selected, seconds);
        text_fit_centered(
            canvas,
            label,
            x + 355.0,
            y + 16.0,
            620.0,
            42.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
        if badge > 0 {
            rr(
                canvas,
                x + 590.0,
                y + 25.0,
                74.0,
                42.0,
                6.0,
                if selected {
                    Color::WHITE
                } else {
                    Color::rgb(95, 101, 108)
                },
            );
            canvas.text_centered(
                &badge.to_string(),
                x + 627.0,
                y + 30.0,
                28.0,
                if selected {
                    theme.primary_color()
                } else {
                    Color::WHITE
                },
                FontWeight::Bold,
            );
        }
    }
    crate::ui::ipod_scrollbar(canvas, 1222.0, 82.0, 820.0, 8, 9, index_for_focus(focus));
}

fn index_for_focus(focus: Focus) -> usize {
    match focus {
        Focus::Search => 0,
        Focus::Albums => 1,
        Focus::Songs => 2,
        Focus::Artists => 3,
        Focus::Playlists => 4,
        Focus::Favorites => 5,
        Focus::Queue => 6,
        Focus::History => 7,
        Focus::ShuffleAll => 8,
        _ => 0,
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
        draw_empty_collection(
            canvas,
            "Nothing here yet",
            "Add music, then run a library scan.",
        );
        return;
    }

    for (idx, item) in items.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 260.0 + idx as f32 * 144.0 - scroll_offset;

        if y < 252.0 || y + 128.0 > 862.0 {
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
        draw_empty_collection(
            canvas,
            "No songs yet",
            "Sync from the desktop or add files to /boot/music.",
        );
        return;
    }

    for (idx, track) in tracks.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 260.0 + idx as f32 * 144.0 - scroll_offset;

        if y < 252.0 || y + 128.0 > 862.0 {
            continue;
        }

        let subtitle = if track.rating > 0 {
            format!("{}  |  Rating {}/5", track.artist, track.rating)
        } else {
            track.artist.clone()
        };
        collection_row(
            canvas,
            y,
            &track.title,
            &subtitle,
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
}

fn draw_empty_collection(canvas: &mut Canvas<'_>, title: &str, help: &str) {
    liquid_glass(canvas, 238.0, 330.0, 824.0, 300.0, 32.0, 190);
    canvas.text_centered(title, 650.0, 392.0, 68.0, Color::BLACK, FontWeight::Bold);
    text_fit_centered(
        canvas,
        help,
        650.0,
        500.0,
        680.0,
        34.0,
        Color::rgb(70, 70, 70),
        FontWeight::Regular,
    );
    canvas.text_centered(
        "B  Back",
        650.0,
        570.0,
        28.0,
        Color::rgba(0, 0, 0, 140),
        FontWeight::Regular,
    );
}

pub fn draw_search(
    canvas: &mut Canvas<'_>,
    query: &str,
    results: &[Track],
    key_index: usize,
    selected_idx: usize,
    focus: Focus,
    theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    if focus == Focus::SearchResults {
        let title = if query.trim().is_empty() {
            "Search Results".to_string()
        } else {
            format!("Search: {query}")
        };
        if results.is_empty() {
            draw_collection_shell(canvas, &title, theme);
            draw_empty_collection(canvas, "No matching songs", "Try another title or artist.");
            return;
        }
        draw_tracks_list(
            canvas,
            &title,
            results,
            selected_idx,
            theme,
            scroll_offset,
            transition_t,
            seconds,
        );
        return;
    }

    draw_collection_shell(canvas, "Search Library", theme);
    rr(
        canvas,
        180.0,
        250.0,
        940.0,
        96.0,
        36.0,
        crate::ui::control(canvas),
    );
    text_fit_centered(
        canvas,
        if query.is_empty() {
            "Enter a search"
        } else {
            query
        },
        650.0,
        270.0,
        850.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        &format!("{} results", results.len()),
        650.0,
        360.0,
        28.0,
        Color::rgb(70, 70, 70),
        FontWeight::Regular,
    );

    const KEYS: [&str; 30] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z", "SPACE", "DEL", "CLEAR", "RESULTS",
    ];
    for (index, label) in KEYS.iter().enumerate() {
        let col = index % 6;
        let row = index / 6;
        let x = 188.0 + col as f32 * 153.0;
        let y = 418.0 + row as f32 * 82.0;
        let selected = index == key_index;
        let w = 136.0;
        let h = 66.0;
        rr(
            canvas,
            x,
            y,
            w,
            h,
            22.0,
            if selected {
                crate::ui::accent(canvas)
            } else {
                crate::ui::control(canvas)
            },
        );
        if selected {
            crate::ui::register_focus(canvas, x, y, w, h, 22.0);
        }
        canvas.text_centered(
            label,
            x + w / 2.0,
            y + 17.0,
            if label.len() > 3 { 24.0 } else { 30.0 },
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
    }
}

pub fn draw_rating_picker(
    canvas: &mut Canvas<'_>,
    title: &str,
    rating: u8,
    theme: ThemeName,
    _seconds: f32,
) {
    draw_collection_shell(canvas, "Rate Track", theme);
    text_fit_centered(
        canvas,
        title,
        650.0,
        300.0,
        900.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    for index in 0..5 {
        let active = index < rating as usize;
        let x = 325.0 + index as f32 * 130.0;
        rr(
            canvas,
            x,
            450.0,
            104.0,
            104.0,
            52.0,
            if active {
                crate::ui::accent(canvas)
            } else {
                crate::ui::control(canvas)
            },
        );
        canvas.text_centered(
            "★",
            x + 52.0,
            469.0,
            57.0,
            if active {
                Color::WHITE
            } else {
                Color::rgb(100, 100, 100)
            },
            FontWeight::Bold,
        );
    }
    canvas.register_selected(305.0, 430.0, 670.0, 144.0, 72.0, 6.0);
    canvas.text_centered(
        if rating == 0 {
            "Unrated"
        } else {
            "Confirm rating"
        },
        650.0,
        620.0,
        36.0,
        Color::rgb(70, 70, 70),
        FontWeight::Regular,
    );
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
    let _ = theme;
    panel_gradient(
        canvas,
        32.0,
        78.0,
        1216.0,
        850.0,
        8.0,
        Color::WHITE,
        Color::WHITE,
    );
    text_fit_centered(
        canvas,
        title,
        640.0,
        91.0,
        960.0,
        58.0,
        Color::BLACK,
        FontWeight::Bold,
    );
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
    let x = 72.0;
    let w = 1136.0;
    let h = 112.0;
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
    let y = y + (1.0 - row_t) * 46.0;
    figma_list_row(canvas, x, y, w, h, selected, seconds);

    let text_color = if selected { Color::WHITE } else { Color::BLACK };

    crate::ui::text_fit(
        canvas,
        title,
        x + 26.0,
        y + if subtitle.is_empty() { 26.0 } else { 14.0 },
        940.0,
        52.0,
        text_color,
        FontWeight::Bold,
    );
    if !subtitle.is_empty() {
        crate::ui::text_fit(
            canvas,
            subtitle,
            x + 28.0,
            y + 67.0,
            880.0,
            28.0,
            text_color,
            FontWeight::Regular,
        );
    }
    canvas.text_right(
        "›",
        x + w - 26.0,
        y + 26.0,
        54.0,
        text_color,
        FontWeight::Bold,
    );
}
