use crate::app::Focus;
use crate::database::{LibraryCounts, Pin, Track};
use crate::renderer::Canvas;
use crate::ui::library::LibraryView;
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{compact_row, rr, FontWeight};

pub fn draw_music_menu(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    _seconds: f32,
    lang: Language,
    theme: ThemeName,
    pins: &[Pin],
    counts: LibraryCounts,
    queue_len: usize,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    let _ = theme;
    crate::ui::draw_figma_background(canvas);
    let pin_summary = pins
        .iter()
        .take(4)
        .map(|pin| pin.name.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    if !pin_summary.is_empty() {
        crate::ui::page_heading(canvas, lang.translate("pins"), &pin_summary);
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

    for (item_focus, label, badge, _) in items {
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
        let x = 44.0 - (1.0 - row_t) * 26.0;
        let y = if pin_summary.is_empty() { 92.0 } else { 206.0 }
            + index as f32 * if pin_summary.is_empty() { 84.0 } else { 73.0 };
        let selected = focus == item_focus;
        compact_row(canvas, x, y, 1192.0, 68.0, selected);
        crate::ui::text_fit(
            canvas,
            label,
            x + 24.0,
            y + 16.0,
            1040.0,
            36.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
        if badge > 0 {
            canvas.text_right(
                &badge.to_string(),
                x + 1158.0,
                y + 18.0,
                30.0,
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
    crate::ui::draw_figma_background(canvas);

    if items.is_empty() {
        draw_empty_collection(
            canvas,
            "Nothing here yet",
            "Add music, then run a library scan.",
        );
        return;
    }

    crate::ui::page_heading(canvas, title, &format!("{} items", items.len()));
    for (idx, item) in items.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 220.0 + idx as f32 * COLLECTION_ROW_STEP - scroll_offset;

        if y < 204.0 || y + COLLECTION_ROW_HEIGHT > 864.0 {
            continue;
        }

        collection_row(
            canvas,
            y,
            item,
            "",
            None,
            None,
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

pub(crate) fn draw_tracks_list(
    canvas: &mut Canvas<'_>,
    library: &LibraryView,
    title: &str,
    tracks: &[Track],
    selected_idx: usize,
    _theme: ThemeName,
    scroll_offset: f32,
    transition_t: f32,
    seconds: f32,
) {
    crate::ui::draw_figma_background(canvas);

    if tracks.is_empty() {
        draw_empty_collection(
            canvas,
            "No songs yet",
            "Sync from the desktop or add files to /boot/music.",
        );
        return;
    }

    crate::ui::page_heading(canvas, title, &format!("{} tracks", tracks.len()));
    for (idx, track) in tracks.iter().enumerate() {
        let selected = idx == selected_idx;
        let y = 220.0 + idx as f32 * COLLECTION_ROW_STEP - scroll_offset;

        if y < 204.0 || y + COLLECTION_ROW_HEIGHT > 864.0 {
            continue;
        }

        let mut subtitle = format!("{} · {}", track.artist, track.album);
        if track.rating > 0 {
            subtitle.push_str(&format!(" · {}/5", track.rating));
        }
        let duration = track.duration.max(0);
        collection_row(
            canvas,
            y,
            &format!("{:02}  {}", idx + 1, track.title),
            &subtitle,
            Some(format!("{}:{:02}", duration / 60, duration % 60)),
            Some((library, track.cover_art_path.as_deref())),
            selected,
            idx,
            transition_t,
            seconds,
        );
    }
}

fn draw_empty_collection(canvas: &mut Canvas<'_>, title: &str, help: &str) {
    crate::ui::page_heading(canvas, title, help);
}

pub(crate) fn draw_search(
    canvas: &mut Canvas<'_>,
    library: &LibraryView,
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
            crate::ui::draw_figma_background(canvas);
            draw_empty_collection(canvas, "No matching songs", "Try another title or artist.");
            return;
        }
        draw_tracks_list(
            canvas,
            library,
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

    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(
        canvas,
        if query.is_empty() {
            "Search Library"
        } else {
            query
        },
        &format!("{} results", results.len()),
    );
    const KEYS: [&str; 30] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z", "SPACE", "DEL", "CLEAR", "RESULTS",
    ];
    for (index, label) in KEYS.iter().enumerate() {
        let col = index % 6;
        let row = index / 6;
        let x = 48.0 + col as f32 * 199.0;
        let y = 220.0 + row as f32 * 124.0;
        let selected = index == key_index;
        let w = 182.0;
        let h = 97.0;
        rr(
            canvas,
            x,
            y,
            w,
            h,
            12.0,
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
            if label.len() > 3 { 24.0 } else { 34.0 },
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
    crate::ui::draw_figma_background(canvas);
    crate::ui::page_heading(canvas, "Rate Track", title);
    for index in 0..5 {
        let active = index < rating as usize;
        let x = 340.0 + index as f32 * 128.0;
        rr(
            canvas,
            x,
            450.0,
            104.0,
            104.0,
            14.0,
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

    canvas.text_centered(
        if rating == 0 {
            "Unrated"
        } else {
            "Confirm rating"
        },
        640.0,
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
    let step = (560.0 / letters.len().max(1) as f32).clamp(31.0, 42.0);
    let mut y = 229.0;
    for letter in letters.into_iter().take(18) {
        canvas.text_centered(
            &letter.to_string(),
            1212.0,
            y,
            30.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Bold,
        );
        y += step;
    }
}

pub(crate) const COLLECTION_ROW_STEP: f32 = 90.0;
pub(crate) const COLLECTION_ROW_HEIGHT: f32 = 84.0;

fn collection_row(
    canvas: &mut Canvas<'_>,
    y: f32,
    title: &str,
    subtitle: &str,
    trailing: Option<String>,
    artwork: Option<(&LibraryView, Option<&str>)>,
    selected: bool,
    index: usize,
    transition_t: f32,
    _seconds: f32,
) {
    let x = 44.0;
    let w = 1138.0;
    let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
    let x = x - (1.0 - row_t) * 24.0;
    compact_row(canvas, x, y, w, COLLECTION_ROW_HEIGHT, selected);
    let text_x = if let Some((library, path)) = artwork {
        library.artwork(canvas, path, x + 20.0, y + 12.0, 60.0, false);
        x + 100.0
    } else {
        x + 20.0
    };
    let text_width = x + w - 142.0 - text_x;
    crate::ui::text_fit(
        canvas,
        title,
        text_x,
        y + if subtitle.is_empty() { 20.0 } else { 3.0 },
        text_width,
        40.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    if !subtitle.is_empty() {
        crate::ui::text_fit(
            canvas,
            subtitle,
            text_x,
            y + 42.0,
            text_width,
            36.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Regular,
        );
    }
    canvas.text_right(
        trailing.as_deref().unwrap_or("›"),
        x + w - 20.0,
        y + 23.0,
        31.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Regular,
    );
}
