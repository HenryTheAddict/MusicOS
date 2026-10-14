#include "../Components.hpp"
#include <iostream>
#include <cmath>
#include <algorithm>

namespace MusicUI {

// 5. Puffy Toggle (Segmented Control)
PuffyToggle::PuffyToggle(const Rect& b) : Widget(b) {}

void PuffyToggle::render_self(Canvas& canvas) {
    // Outer container -> fill #D6DEFD, radius 16
    canvas.fill_rounded_rect(bounds, 16.0f, Color::from_hex(0xD6DEFD));

    // Inner track -> fill #6282FF, radius 16
    float track_w = bounds.w - 7.0f;
    Rect track_r(bounds.x + 4.0f, bounds.y + 4.0f, track_w, 45.0f);
    canvas.fill_rounded_rect(track_r, 16.0f, Color::from_hex(0x6282FF));

    float half_w = track_w * 0.5f;
    float target_x = (selection == Selection::AutoManage) ? 0.0f : half_w;
    anim_x.set_target(target_x);
    float stretch = std::min(10.0f, std::abs(anim_x.vel) * 0.012f);
    Rect active_pill(
        track_r.x + anim_x.val - (anim_x.vel > 0 ? 0.0f : stretch),
        track_r.y,
        half_w + stretch,
        45.0f
    );
    canvas.fill_rounded_rect(active_pill, 16.0f, Color::from_hex(0x62D2FF));

    // Left option: Auto-Manage
    float left_cx = track_r.x + half_w * 0.5f;
    Rect auto_icon_r(left_cx - 68.0f, track_r.y + 10.0f, 24.0f, 24.0f);
    canvas.draw_svg("assets/icons/sketch/AutoStore.svg", auto_icon_r);
    canvas.draw_text("Auto-Manage", Point(left_cx - 36.0f, track_r.y + 14.0f), 15.0f, Color::Black, FontWeight::DemiBold, TextAlign::Left);

    // Right option: Manual Manage
    float right_cx = track_r.x + half_w + half_w * 0.5f;
    Rect manual_icon_r(right_cx - 76.0f, track_r.y + 10.0f, 24.0f, 24.0f);
    canvas.draw_svg("assets/icons/sketch/Storage.svg", manual_icon_r);
    canvas.draw_text("Manual Manage", Point(right_cx - 44.0f, track_r.y + 14.0f), 15.0f, Color::Black, FontWeight::Light, TextAlign::Left);
}

bool PuffyToggle::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;

    if (x < bounds.x + bounds.w * 0.5f) {
        selection = Selection::AutoManage;
    } else {
        selection = Selection::ManualManage;
    }
    if (on_selection_change) on_selection_change(selection);
    return true;
}

// 6. Category Tab Bar
CategoryTabBar::CategoryTabBar(const Rect& b) : Widget(b) {}

void CategoryTabBar::render_self(Canvas& canvas) {
    canvas.fill_rounded_rect(bounds, 8.0f, Color::from_hex(0xB6C5FF));
    const char* tabs[4] = { "Artists", "Albums", "Playlists", "Songs" };
    float tab_w = bounds.w / 4.0f;
    const float tab_cx[4] = { 51.25f, 51.25f, 52.0f, 51.25f };

    float target_x = selected_index * tab_w;
    anim_x.set_target(target_x);
    Rect active_tab(bounds.x + anim_x.val, bounds.y, tab_w, bounds.h);
    canvas.fill_rounded_rect(active_tab, 8.0f, Color::from_hex(0x88A1FF));

    for (int i = 0; i < 4; ++i) {
        Rect tab_r(bounds.x + i * tab_w, bounds.y, tab_w, bounds.h);
        canvas.draw_text(tabs[i], Point(tab_r.x + tab_cx[i], tab_r.y + 7.5f), 16.0f, Color::Black, FontWeight::Light, TextAlign::Center);
    }
}

bool CategoryTabBar::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;
    float tab_w = bounds.w / 4.0f;
    int idx = (int)((x - bounds.x) / tab_w);
    if (idx >= 0 && idx < 4) {
        selected_index = idx;
        if (on_tab_change) on_tab_change(selected_index);
        return true;
    }
    return false;
}

// 7. Qualitizer Card
QualitizerCard::QualitizerCard(const Rect& b) : Widget(b) {}

void QualitizerCard::render_self(Canvas& canvas) {
    canvas.fill_rounded_rect(bounds, 16.0f, Color::from_hex(0xD6DEFD));
    canvas.draw_text("Qualitizer", Point(bounds.x + 56.0f, bounds.y + 4.0f), 24.0f, Color::Black, FontWeight::Light, TextAlign::Center);

    // Track: (4, 30, bounds.w - 8, 48), fill #88A1FF
    Rect track_r(bounds.x + 4.0f, bounds.y + 30.0f, bounds.w - 8.0f, 48.0f);
    canvas.fill_rounded_rect(track_r, 8.0f, Color::from_hex(0x88A1FF));

    float seg_w = track_r.w / 4.0f;
    float target_x = selected_rate * seg_w;
    anim_x.set_target(target_x);
    Rect active_pill(track_r.x + anim_x.val, track_r.y, seg_w, 48.0f);
    canvas.fill_rounded_rect(active_pill, 8.0f, Color::from_hex(0x007FFF));

    // 4 segment buttons
    for (int i = 0; i < 4; ++i) {
        Rect seg_r(track_r.x + i * seg_w, track_r.y, seg_w, 48.0f);
        float cx = seg_r.x + seg_w * 0.5f;
        if (i == 0) {
            Color text_col = (selected_rate == 0) ? Color::Black : Color::White;
            canvas.draw_text("Lossless/OG", Point(cx, seg_r.y + 16.0f), 16.0f, text_col, FontWeight::Light, TextAlign::Center);
        } else {
            std::string sub = (i == 1) ? "128/kbps" : ((i == 2) ? "256/kbps" : "320/kbps");
            Color text_col = (selected_rate == i) ? Color::Black : Color::White;
            canvas.draw_text("Opus", Point(cx, seg_r.y + 8.0f), 16.0f, text_col, FontWeight::Light, TextAlign::Center);
            canvas.draw_text(sub, Point(cx, seg_r.y + 24.0f), 16.0f, text_col, FontWeight::Light, TextAlign::Center);
        }
    }
}

bool QualitizerCard::handle_mouse_down(float x, float y, int /*button*/) {
    Rect track_r(bounds.x + 4.0f, bounds.y + 30.0f, bounds.w - 8.0f, 48.0f);
    if (!track_r.contains(x, y)) return false;

    float seg_w = track_r.w / 4.0f;
    int idx = (int)((x - track_r.x) / seg_w);
    if (idx >= 0 && idx < 4) {
        selected_rate = idx;
        if (on_rate_change) on_rate_change(selected_rate);
        return true;
    }
    return false;
}

// 8. Manual Sync Management Card
ManualSyncCard::ManualSyncCard(const Rect& b) : Widget(b) {
    tab_bar = std::make_shared<CategoryTabBar>(Rect(bounds.x + 8.0f, bounds.y + 8.0f, bounds.w - 16.0f, 32.0f));
}

void ManualSyncCard::render_self(Canvas& canvas) {
    canvas.fill_rounded_rect(bounds, 16.0f, Color::from_hex(0xD6DEFD));

    // Tab bar at (8, 8, 412, 32)
    if (tab_bar) tab_bar->render(canvas);

    // Search Pill: (8, 44, 204, 24), fill #B6C5FF
    Rect search_r(bounds.x + 8.0f, bounds.y + 44.0f, 204.0f, 24.0f);
    canvas.fill_rounded_rect(search_r, 8.0f, Color::from_hex(0xB6C5FF));
    canvas.draw_text("Search", Point(search_r.x + 102.0f, search_r.y + 4.0f), 16.0f, Color::Black, FontWeight::Light, TextAlign::Center);

    // Filter Pill: (216, 44, 204, 24), fill #B6C5FF
    Rect filter_r(bounds.x + 216.0f, bounds.y + 44.0f, 204.0f, 24.0f);
    canvas.fill_rounded_rect(filter_r, 8.0f, Color::from_hex(0xB6C5FF));
    canvas.draw_text("Filter", Point(filter_r.x + 96.0f, filter_r.y + 4.0f), 16.0f, Color::Black, FontWeight::Light, TextAlign::Center);
    canvas.draw_svg("assets/icons/sketch/tinyMinimize.svg", Rect(filter_r.x + 118.0f, filter_r.y + 8.0f, 8.0f, 8.0f));

    // Matrix Container: (8, 72, 412, 132), fill #88A1FF
    Rect matrix_r(bounds.x + 8.0f, bounds.y + 72.0f, bounds.w - 16.0f, 132.0f);
    canvas.fill_rounded_rect(matrix_r, 16.0f, Color::from_hex(0x88A1FF));

    // 8 Item Rows (4 rows x 2 cols):
    // Col 0: x = 17, Col 1: x = 210
    // Rows: y = 5, 41, 77, 113
    float col_x[2] = { 17.0f, 210.0f };
    float row_y[4] = { 5.0f, 41.0f, 77.0f, 113.0f };

    static const char* artist_names[8] = {
        "Gorillaz", "Daft Punk",
        "Tame Impala", "Coldplay",
        "Radiohead", "Twenty One Pilots",
        "Foster the People", "Cage The Elephant"
    };

    canvas.push_clip_rounded_rect(matrix_r, 16.0f);
    for (int r = 0; r < 4; ++r) {
        for (int c = 0; c < 2; ++c) {
            int idx = r * 2 + c;
            Rect item_r(matrix_r.x + col_x[c], matrix_r.y + row_y[r], 185.0f, 32.0f);
            canvas.fill_rounded_rect(item_r, 4.0f, Color::from_hex(0xD6DEFD));

            // Avatar circle: (4, 4, 24, 24) -> radius 12.0f
            canvas.fill_rounded_rect(Rect(item_r.x + 4.0f, item_r.y + 4.0f, 24.0f, 24.0f), 12.0f, Color::from_hex(0x5B5B5B));

            // Text: size 13 Medium centered
            canvas.draw_text(artist_names[idx], Point(item_r.x + 85.0f, item_r.y + 9.0f), 12.0f, Color::Black, FontWeight::Medium, TextAlign::Center);

            // Checkbox: (169, 10, 12, 12)
            bool is_checked = checked[idx];
            Rect chk_r(item_r.x + 169.0f, item_r.y + 10.0f, 12.0f, 12.0f);
            Color chk_col = is_checked ? Color::from_hex(0x0036FF) : Color(0.666f, 0.666f, 0.666f);
            canvas.fill_rounded_rect(chk_r, 3.0f, chk_col);
            canvas.stroke_rounded_rect(chk_r, 3.0f, Color::White, 1.0f);
        }
    }
    canvas.pop_clip();
}

bool ManualSyncCard::handle_mouse_down(float x, float y, int button) {
    if (!bounds.contains(x, y)) return false;

    if (tab_bar && tab_bar->handle_mouse_down(x, y, button)) {
        return true;
    }

    Rect matrix_r(bounds.x + 8.0f, bounds.y + 72.0f, bounds.w - 16.0f, 132.0f);
    if (matrix_r.contains(x, y)) {
        float col_x[2] = { 17.0f, 210.0f };
        float row_y[4] = { 5.0f, 41.0f, 77.0f, 113.0f };
        for (int r = 0; r < 4; ++r) {
            for (int c = 0; c < 2; ++c) {
                Rect item_r(matrix_r.x + col_x[c], matrix_r.y + row_y[r], 185.0f, 32.0f);
                if (item_r.contains(x, y)) {
                    int idx = r * 2 + c;
                    checked[idx] = !checked[idx];
                    int count = 0;
                    for (int i = 0; i < 8; ++i) if (checked[i]) count++;
                    if (on_sync_change) on_sync_change(count);
                    return true;
                }
            }
        }
    }
    return false;
}

} // namespace MusicUI
