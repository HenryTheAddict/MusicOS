use crate::app::Focus;
use crate::database::{LibraryCounts, Pin, Track};
use crate::renderer::Canvas;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{figma_list_row, rr, FontWeight};

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
    let _ = theme;
    crate::ui::rows_preview(canvas);
    canvas.text_centered(
        lang.translate("pins"),
        954.0,
        104.0,
        50.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );

    if pins.is_empty() {
        let help = lang.translate("pins_help");
        let lines: Vec<&str> = help.lines().collect();
        crate::ui::centered_lines(
            canvas,
            &lines,
            954.0,
            228.0,
            36.0,
            36.0,
            crate::ui::ink(canvas),
            FontWeight::Regular,
        );
    } else {
        let limit = pins.len().min(5);
        for i in 0..limit {
            let y = 192.0 + i as f32 * 130.0;

            crate::ui::liquid_glass(canvas, 688.0, y, 530.0, 110.0, 30.0, 160);

            crate::ui::text_fit(
                canvas,
                &pins[i].name,
                712.0,
                y + 16.0,
                480.0,
                36.0,
                crate::ui::ink(canvas),
                FontWeight::Bold,
            );
            crate::ui::text_fit(
                canvas,
                &pins[i].subtitle,
                712.0,
                y + 60.0,
                480.0,
                28.0,
                crate::ui::ink_secondary(canvas),
                FontWeight::Regular,
            );
        }
    }

    let items = [
        (Focus::Search, lang.translate("search"), 0, 68.0),
        (
            Focus::Albums,
            lang.translate("albums"),
            counts.albums,
            156.0,
        ),
        (Focus::Songs, lang.translate("songs"), counts.tracks, 244.0),
        (
            Focus::Artists,
            lang.translate("artists"),
            counts.artists,
            332.0,
        ),
        (
            Focus::Playlists,
            lang.translate("playlists"),
            counts.playlists,
            420.0,
        ),
        (
            Focus::Favorites,
            lang.translate("favorites"),
            counts.favorites,
            508.0,
        ),
        (
            Focus::Queue,
            lang.translate("queue"),
            queue_len as i64,
            596.0,
        ),
        (Focus::History, lang.translate("history"), 0, 684.0),
        (Focus::ShuffleAll, lang.translate("shuffle_all"), 0, 772.0),
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
        let x = 24.0 - (1.0 - row_t) * 35.0;
        let selected = focus == item_focus;
        figma_list_row(canvas, x, y, 600.0, 80.0, selected, seconds);
        crate::ui::text_fit(
            canvas,
            label,
            x + 24.0,
            y + 16.0,
            448.0,
            42.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        if badge > 0 {
            rr(
                canvas,
                x + 505.0,
                y + 25.0,
                74.0,
                42.0,
                6.0,
                crate::ui::surface(canvas),
            );
            canvas.text_centered(
                &badge.to_string(),
                x + 542.0,
                y + 30.0,
                28.0,
                crate::ui::ink_secondary(canvas),
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
    _theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    crate::ui::rows_preview(canvas);

    if items.is_empty() {
        draw_empty_collection(
            canvas,
            "Nothing here yet",
            "Add music, then run a library scan.",
        );
        return;
    }

    let subtitle = format!("{} items", items.len());
    crate::ui::preview_heading(
        canvas,
        items.get(selected_idx).map_or(title, String::as_str),
        &subtitle,
    );
    for (idx, item) in items.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 260.0 + idx as f32 * 144.0 - scroll_offset;

        if y < 252.0 || y + 128.0 > 862.0 {
            continue;
        }

        collection_row(
            canvas,
            y - 100.0,
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
    _theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    crate::ui::rows_preview(canvas);

    if tracks.is_empty() {
        draw_empty_collection(
            canvas,
            "No songs yet",
            "Sync from the desktop or add files to /boot/music.",
        );
        return;
    }

    if let Some(track) = tracks.get(selected_idx) {
        crate::ui::preview_heading(
            canvas,
            &track.title,
            &format!("{} · {}", track.artist, track.album),
        );
        crate::ui::text_fit(
            canvas,
            title,
            688.0,
            244.0,
            532.0,
            28.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Regular,
        );
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
            y - 100.0,
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
    crate::ui::preview_heading(canvas, title, help);
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
            crate::ui::rows_preview(canvas);
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

    crate::ui::rows_preview(canvas);
    crate::ui::preview_heading(
        canvas,
        if query.is_empty() {
            "Search Library"
        } else {
            query
        },
        &format!("{} results", results.len()),
    );
    for (index, track) in results.iter().take(5).enumerate() {
        crate::ui::text_fit(
            canvas,
            &track.title,
            688.0,
            252.0 + index as f32 * 100.0,
            532.0,
            32.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        crate::ui::text_fit(
            canvas,
            &track.artist,
            688.0,
            290.0 + index as f32 * 100.0,
            532.0,
            26.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Regular,
        );
    }

    const KEYS: [&str; 30] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z", "SPACE", "DEL", "CLEAR", "RESULTS",
    ];
    for (index, label) in KEYS.iter().enumerate() {
        let col = index % 6;
        let row = index / 6;
        let x = 24.0 + col as f32 * 102.0;
        let y = 167.0 + row as f32 * 121.0;
        let selected = index == key_index;
        let w = 90.0;
        let h = 97.0;
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
            y + 29.0,
            if label.len() > 3 { 19.0 } else { 30.0 },
            if selected {
                crate::ui::on_accent(canvas)
            } else {
                crate::ui::ink(canvas)
            },
            FontWeight::Bold,
        );
    }
}

pub fn draw_rating_picker(
    canvas: &mut Canvas<'_>,
    title: &str,
    rating: u8,
    _theme: ThemeName,
    _seconds: f32,
) {
    crate::ui::rows_preview(canvas);
    crate::ui::preview_heading(canvas, "Rate Track", title);
    for index in 0..5 {
        let active = index < rating as usize;
        let x = 35.0 + index as f32 * 115.0;
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
                crate::ui::on_accent(canvas)
            } else {
                crate::ui::ink_secondary(canvas)
            },
            FontWeight::Bold,
        );
    }
    canvas.register_selected(25.0, 430.0, 585.0, 144.0, 72.0, 6.0);
    canvas.text_centered(
        if rating == 0 {
            "Unrated"
        } else {
            "Confirm rating"
        },
        320.0,
        620.0,
        36.0,
        crate::ui::ink_secondary(canvas),
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
            620.0,
            y - 100.0,
            22.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Bold,
        );
        y += step;
    }
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
    let x = 24.0;
    let w = 568.0;
    let h = 112.0;
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
    let y = y + (1.0 - row_t) * 46.0;
    figma_list_row(canvas, x, y, w, h, selected, seconds);

    let text_color = crate::ui::ink(canvas);

    crate::ui::text_fit(
        canvas,
        title,
        x + 26.0,
        y + if subtitle.is_empty() { 26.0 } else { 14.0 },
        480.0,
        43.0,
        text_color,
        FontWeight::Bold,
    );
    if !subtitle.is_empty() {
        crate::ui::text_fit(
            canvas,
            subtitle,
            x + 28.0,
            y + 67.0,
            460.0,
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
