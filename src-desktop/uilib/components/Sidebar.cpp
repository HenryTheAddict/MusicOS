#include "../Components.hpp"
#include "../LibraryModel.hpp"
#include <iostream>
#include <cmath>
#include <algorithm>

namespace MusicUI {

Sidebar::Sidebar(const Rect& b) : Widget(b) {}

void Sidebar::render_self(Canvas& canvas) {
    if (mode == Mode::Library) {
        canvas.fill_rounded_rect(bounds, 16.0f, Color::from_hex(0xCBF1FF));

        canvas.draw_text("Library", Point(bounds.x + 76.0f, bounds.y + 3.0f), 14.0f, Color::Black, FontWeight::Medium);

        // 4 Nav Rows
        const char* navs[4] = { "Albums", "Artists", "Genres", "Songs" };
        const char* nav_icons[4] = {
            "assets/icons/sketch/Album.svg",
            "assets/icons/sketch/Artist.svg",
            "assets/icons/sketch/Genre.svg",
            "assets/icons/sketch/Song.svg"
        };
        float row_y[4] = { 20.0f, 54.0f, 88.0f, 122.0f };

        for (int i = 0; i < 4; ++i) {
            Rect nrow(bounds.x, bounds.y + row_y[i], bounds.w, 32.0f);
            Color row_col = (i == nav_selected) ? Color::from_hex(0x009AFF) : Color::from_hex(0xAAE4FF);
            canvas.fill_rounded_rect(nrow, 8.0f, row_col, 0.6f);

            canvas.draw_svg(nav_icons[i], Rect(nrow.x, nrow.y, 32.0f, 32.0f));
            const float nav_x[4] = { 72.0f, 78.0f, 74.0f, 77.0f };
            const FontWeight nav_w[4] = { FontWeight::DemiBold, FontWeight::Medium, FontWeight::Medium, FontWeight::Medium };
            canvas.draw_text(i == 0 && !back_label.empty() ? back_label : navs[i], Point(nrow.x + (i == 0 && !back_label.empty() ? 36.0f : nav_x[i]), nrow.y + 9.0f), 14.0f, Color::Black, nav_w[i]);

            if (i == nav_selected || (i == 0 && !back_label.empty())) {
                canvas.draw_svg(i == 0 && !back_label.empty() ? "assets/icons/sketch/Back.svg" : "assets/icons/sketch/3dots_(nocircle).svg", Rect(nrow.x + 164.0f, nrow.y, 32.0f, 32.0f));
            }
        }

        // "New:" container Frame 2: (0, 156, 196, 64), fill #00BAFF alpha 0.5, radius 16
        Rect new_cont(bounds.x, bounds.y + 156.0f, bounds.w, 64.0f);
        canvas.fill_rounded_rect(new_cont, 16.0f, Color(0.0f, 0.7302f, 1.0f, 0.5f));
        canvas.draw_text("New:", Point(new_cont.x + 80.0f, new_cont.y + 7.0f), 14.0f, Color::Black, FontWeight::Medium);

        // Playlist button: (4, 25, 92, 32), radius 11
        Rect pl_btn(new_cont.x + 4.0f, new_cont.y + 25.0f, 92.0f, 32.0f);
        canvas.fill_rounded_rect(pl_btn, 11.0f, Color(0.0f, 0.90f, 1.0f));
        canvas.stroke_rounded_rect(pl_btn, 11.0f, Color(0.0f, 0.0f, 0.0f, 0.5f), 1.0f);
        canvas.draw_svg("assets/icons/sketch/Playlist.svg", Rect(pl_btn.x, pl_btn.y, 32.0f, 32.0f));
        canvas.draw_text("Playlist", Point(pl_btn.x + 30.0f, pl_btn.y + 9.0f), 14.0f, Color::Black, FontWeight::Medium);

        // Sparklist button: (100, 25, 92, 32), radius 11
        Rect sp_btn(new_cont.x + 100.0f, new_cont.y + 25.0f, 92.0f, 32.0f);
        LinearGradient sp_grad(
            Point(sp_btn.x + sp_btn.w * 0.5f, sp_btn.y),
            Point(sp_btn.x + sp_btn.w * 0.5f, sp_btn.bottom()),
            { GradientStop(0.0f, Color(0.0f, 0.86f, 1.0f)), GradientStop(1.0f, Color(0.20f, 0.54f, 1.0f)) }
        );
        canvas.fill_linear_gradient(sp_btn, sp_grad, 11.0f);
        canvas.stroke_rounded_rect(sp_btn, 11.0f, Color(0.0f, 0.0f, 0.0f, 0.5f), 1.0f);
        canvas.draw_svg("assets/icons/sketch/Sparklist.svg", Rect(sp_btn.x, sp_btn.y, 32.0f, 32.0f));
        canvas.draw_text("Sparklist", Point(sp_btn.x + 32.0f, sp_btn.y + 9.0f), 14.0f, Color::Black, FontWeight::Medium);

        // Divider Frame 4: (0, 224, 196, 2), fill #0075FF
        canvas.fill_rect(Rect(bounds.x, bounds.y + 224.0f, bounds.w, 2.0f), Color::from_hex(0x0075FF));

        // Playlists container Frame 6: fills remaining height
        float pl_h = std::max(182.0f, bounds.h - 232.0f);
        Rect pl_cont(bounds.x + 2.0f, bounds.y + 230.0f, 192.0f, pl_h);
        canvas.fill_rounded_rect(pl_cont, 16.0f, Color(0.3295f, 0.7236f, 1.0f));
        // Stroke outside by insetting by -0.5px
        Rect pl_stroke_r = pl_cont.inset(-0.5f, -0.5f);
        canvas.stroke_rounded_rect(pl_stroke_r, 16.5f, Color(0.0f, 0.2549f, 1.0f), 1.0f);

        // Header Frame 3: (0, 0, 192, 18)
        Rect pl_hdr(pl_cont.x, pl_cont.y, pl_cont.w, 18.0f);
        LinearGradient pl_hdr_grad(
            Point(pl_hdr.x + pl_hdr.w * 0.5f, pl_hdr.y),
            Point(pl_hdr.x + pl_hdr.w * 0.5f, pl_hdr.bottom()),
            {
                GradientStop(0.00f, Color(0.0980f, 0.9333f, 0.9961f)),
                GradientStop(0.12f, Color(0.3373f, 0.9333f, 1.0000f)),
                GradientStop(0.47f, Color(0.5059f, 0.8667f, 1.0000f)),
                GradientStop(1.00f, Color(0.3569f, 0.7373f, 1.0000f))
            }
        );
        canvas.push_clip_rounded_rect(pl_cont, 16.0f);
        canvas.fill_linear_gradient(pl_hdr, pl_hdr_grad, 0.0f);
        canvas.pop_clip();
        std::string heading=display_name+"'s stuff:";
        while(canvas.measure_text(heading,12,FontWeight::Medium).w>pl_hdr.w-12 && !heading.empty()){
            size_t end=heading.size()-1;while(end>0 && (static_cast<unsigned char>(heading[end])&0xc0)==0x80)--end;
            heading.erase(end);
        }
        canvas.draw_text(heading, Point(pl_hdr.x + pl_hdr.w * .5f, pl_hdr.y + 3.0f), 12.0f, Color::Black, FontWeight::Medium, TextAlign::Center);

        // Playlist items
        canvas.push_clip_rounded_rect(pl_cont, 16.0f, 0.6f);
        struct PlaylistItem {
            const char* title;
            const char* sub;
            int cover_idx;
            bool is_sparklist;
        };
        static const PlaylistItem pl_items[8] = {
            { "Treelist", "3h21m - 400 songs", 0, false },
            { "My Playlist40", "40m - 10 songs", 1, false },
            { "Favorites", "Sparklist …", 2, true },
            { "My Playlist8", "40m - 10 songs", 3, false },
            { "Neo Tokyo", "1h15m - 22 songs", 0, false },
            { "Synthwave Rush", "52m - 14 songs", 1, false },
            { "Daft Funk Mix", "1h45m - 25 songs", 2, true },
            { "Late Night", "2h10m - 32 songs", 3, false }
        };
        int max_items = std::min(8, std::max(4, (int)((pl_cont.h - 22.0f) / 44.0f)));

        for (int i = 0; i < max_items; ++i) {
            float y_pos = 20.0f + i * 44.0f;
            Rect prow(pl_cont.x + 2.0f, pl_cont.y + y_pos, 176.0f, 41.0f);
            canvas.fill_rounded_rect(prow, 8.0f, Color(0.8893f, 0.8893f, 0.8893f, 0.5f));

            // Square cover: (4, 5, 32, 32)
            Rect pcover(prow.x + 4.0f, prow.y + 5.0f, 32.0f, 32.0f);
            canvas.draw_image("assets/covers/playlists/pl_" + std::to_string(pl_items[i].cover_idx) + ".png", pcover, 4.0f);

            // Title (RodinNTLGPro-EB in Sketch)
            std::string title=pl_items[i].title;
            float title_width=pl_items[i].is_sparklist?80.f:98.f;
            if(canvas.measure_text(title,12,FontWeight::ExtraBold).w>title_width){
                while(!title.empty() && canvas.measure_text(title+"…",12,FontWeight::ExtraBold).w>title_width)erase_character(title);
                title+="…";
            }
            canvas.draw_text(title, Point(prow.x + 38.0f, prow.y + 5.0f), 12.0f, Color::Black, FontWeight::ExtraBold);

            // Sparklist icon next to Favorites title
            if (pl_items[i].is_sparklist) {
                canvas.draw_svg("assets/icons/sketch/Sparklist.svg", Rect(prow.x + 120.0f, prow.y + 4.0f, 16.0f, 16.0f));
            }

            // Blue line divider: (39, 21, 99, 2)
            canvas.fill_rect(Rect(prow.x + 39.0f, prow.y + 21.0f, 99.0f, 2.0f), Color(0.0f, 0.253f, 1.0f));

            // Subtitle: (39, 26)
            canvas.draw_text(pl_items[i].sub, Point(prow.x + 39.0f, prow.y + 26.0f), 8.0f, Color::Black, FontWeight::Light);

            // ShufflePlay icon: (140, 5, 32, 32)
            Rect sp_box(prow.x + 140.0f, prow.y + 5.0f, 32.0f, 32.0f);
            if (pl_items[i].is_sparklist) {
                // Favorites has blurred rainbow gradient strip baked with 100% parity from Sketch
                canvas.draw_image("assets/icons/sketch/ShufflePlay_rainbow@2x.png", sp_box, 0.0f);
            } else {
                Color stop1 = (i % 2 == 0) ? Color(0.7604f, 1.0f, 0.7854f) : Color(1.0f, 0.8859f, 0.7604f);
                LinearGradient sp_grad(Point(sp_box.x + sp_box.w * 0.5f, sp_box.y), Point(sp_box.x + sp_box.w * 0.5f, sp_box.bottom()), { GradientStop(0.0f, Color(1.0f, 1.0f, 1.0f)), GradientStop(1.0f, stop1) });
                canvas.fill_linear_gradient(sp_box, sp_grad, 8.0f, 0.6f);
                canvas.draw_svg("assets/icons/sketch/ShufflePlay.svg", sp_box);
            }
        }

        // Scrollbar track: (181, 17, 8, track_h)
        float track_h = std::max(60.0f, pl_cont.h - 26.0f);
        Rect sb_track(pl_cont.x + 181.0f, pl_cont.y + 17.0f, 8.0f, track_h);
        canvas.fill_rounded_rect(sb_track, 4.0f, Color(0.4508f, 0.6254f, 1.0f));
        Rect sb_track_stroke = sb_track.inset(-0.5f, -0.5f);
        canvas.stroke_rounded_rect(sb_track_stroke, 4.5f, Color(0.0f, 0.0974f, 0.3701f), 1.0f);

        // Scrollbar thumb
        float thumb_h = std::max(28.0f, track_h * 0.28f);
        Rect sb_thumb(pl_cont.x + 181.0f, pl_cont.y + 17.0f, 8.0f, thumb_h);
        canvas.fill_rounded_rect(sb_thumb, 4.0f, Color(0.0f, 0.2833f, 1.0f));

        canvas.pop_clip();
    } else {
        // Frame 4 (Devices Sidebar): (2, 100, 255, 378), fill #CBF1FF
        canvas.fill_rounded_rect(bounds, 16.0f, Color::from_hex(0xCBF1FF));

        // Device Header
        canvas.draw_svg("assets/icons/sketch/device.svg", Rect(bounds.x + 11.0f, bounds.y + 10.0f, 32.0f, 32.0f));
        canvas.draw_text("Henry Music 01", Point(bounds.x + 43.0f, bounds.y + 10.0f), 16.0f, Color::Black, FontWeight::DemiBold);
        canvas.draw_text(storage_text, Point(bounds.x + 43.0f, bounds.y + 30.0f), 12.0f, Color(0.0f, 0.0f, 0.0f, 0.5f), FontWeight::DemiBold);
        canvas.draw_svg("assets/icons/sketch/3dots(incircle).svg", Rect(bounds.x + 171.0f, bounds.y + 10.0f, 16.0f, 16.0f));

        // Storage Card Frame 3: (11, 50, 232, 42), fill #00C3FF
        Rect st_card(bounds.x + 11.0f, bounds.y + 50.0f, 232.0f, 42.0f);
        canvas.fill_rounded_rect(st_card, 4.0f, Color::from_hex(0x00C3FF));
        // Storage track (0, 0, 232, 24) fill #00E5FF & fill (0, 0, storage_anim.val, 24) fill #0051FF
        Rect st_track(st_card.x, st_card.y, st_card.w, 24.0f);
        canvas.push_clip_rounded_rect(st_card, 4.0f);
        canvas.fill_rect(st_track, Color(0.0f, 0.8988f, 1.0f));
        canvas.fill_rect(Rect(st_track.x, st_track.y, storage_anim.val, 24.0f), Color(0.0f, 0.3161f, 1.0f));
        canvas.pop_clip();
        canvas.draw_text("Music", Point(st_card.x + 5.0f, st_card.y + 26.0f), 14.0f, Color::Black, FontWeight::DemiBold);
        canvas.draw_text("Free", Point(st_card.x + 192.0f, st_card.y + 25.0f), 16.0f, Color::Black, FontWeight::DemiBold);

        // Quick(er) Settings Card Frame 4: (11, 103, 232, 103), fill #81C7FF
        Rect qs_card(bounds.x + 11.0f, bounds.y + 103.0f, 232.0f, 103.0f);
        canvas.fill_rounded_rect(qs_card, 16.0f, Color::from_hex(0x81C7FF));
        canvas.draw_text("Quick(er) Settings", Point(qs_card.x + 8.0f, qs_card.y + 7.0f), 14.0f, Color::Black, FontWeight::DemiBold);

        Rect qs1(qs_card.x + 14.0f, qs_card.y + 27.0f, 96.0f, 32.0f);
        Rect qs2(qs_card.x + 122.0f, qs_card.y + 27.0f, 96.0f, 32.0f);
        Rect qs3(qs_card.x + 14.0f, qs_card.y + 67.0f, 96.0f, 32.0f);
        Rect qs4(qs_card.x + 122.0f, qs_card.y + 67.0f, 96.0f, 32.0f);

        canvas.fill_rounded_rect(qs1, 8.0f, Color::from_hex(0x476CFF));
        canvas.stroke_rounded_rect(qs1, 8.0f, Color::from_hex(0x003CFF), 1.0f);
        canvas.draw_svg("assets/icons/sketch/Lossless_blue.svg", Rect(qs1.x, qs1.y, 32.0f, 32.0f));
        canvas.draw_text("Lossless", Point(qs1.x + 32.0f, qs1.y + 9.0f), 14.0f, Color::Black, FontWeight::DemiBold);

        canvas.fill_rounded_rect(qs2, 8.0f, Color::from_hex(0x476CFF));
        canvas.stroke_rounded_rect(qs2, 8.0f, Color::from_hex(0x003CFF), 1.0f);
        canvas.draw_svg("assets/icons/sketch/NetSearch_blue.svg", Rect(qs2.x, qs2.y, 32.0f, 32.0f));
        canvas.draw_text("Internet", Point(qs2.x + 32.0f, qs2.y + 9.0f), 14.0f, Color::Black, FontWeight::DemiBold);

        canvas.fill_rounded_rect(qs3, 8.0f, Color::from_hex(0xD2DBFF));
        canvas.stroke_rounded_rect(qs3, 8.0f, Color::White, 1.0f);
        canvas.draw_svg("assets/icons/sketch/File.svg", Rect(qs3.x, qs3.y, 32.0f, 32.0f));
        canvas.draw_text("Auto-Sync", Point(qs3.x + 32.0f, qs3.y + 11.0f), 10.0f, Color::Black, FontWeight::DemiBold);

        canvas.fill_rounded_rect(qs4, 8.0f, Color::from_hex(0xD2DBFF));
        canvas.stroke_rounded_rect(qs4, 8.0f, Color::White, 1.0f);
        canvas.draw_svg("assets/icons/sketch/Sparklist.svg", Rect(qs4.x, qs4.y, 32.0f, 32.0f));
        canvas.draw_text("Sparklist", Point(qs4.x + 32.0f, qs4.y + 6.0f), 10.0f, Color::Black, FontWeight::DemiBold);
        canvas.draw_text("AutoUpdate", Point(qs4.x + 32.0f, qs4.y + 16.0f), 10.0f, Color::Black, FontWeight::DemiBold);

        // Audio Settings Card Frame 4: (11, 210, 232, 70), fill #81C7FF
        Rect as_card(bounds.x + 11.0f, bounds.y + 210.0f, 232.0f, 70.0f);
        canvas.fill_rounded_rect(as_card, 16.0f, Color::from_hex(0x81C7FF));
        canvas.draw_text("Audio Settings", Point(as_card.x + as_card.w * 0.5f, as_card.y + 4.0f), 14.0f, Color::Black, FontWeight::DemiBold, TextAlign::Center);

        Rect as1(as_card.x + 16.0f, as_card.y + 22.0f, 96.0f, 44.0f);
        Rect as2(as_card.x + 120.0f, as_card.y + 22.0f, 96.0f, 44.0f);
        canvas.fill_rounded_rect(as1, 16.0f, Color::from_hex(0x00ACFF));
        canvas.stroke_rounded_rect(as1, 16.0f, Color::from_hex(0x5F80FF), 1.0f);
        canvas.draw_text("Quality", Point(as1.x + as1.w * 0.5f, as1.y + 14.0f), 14.0f, Color::Black, FontWeight::UltraBold, TextAlign::Center);

        canvas.fill_rounded_rect(as2, 16.0f, Color::from_hex(0x5F80FF));
        canvas.stroke_rounded_rect(as2, 16.0f, Color::from_hex(0x00ACFF), 1.0f);
        canvas.draw_text("EQ", Point(as2.x + as2.w * 0.5f, as2.y + 15.0f), 14.0f, Color::White, FontWeight::UltraBold, TextAlign::Center);

        // Battery Card Frame 5: (11, 284, 232, 83), fill #000000
        Rect bat_card(bounds.x + 11.0f, bounds.y + 284.0f, 232.0f, 83.0f);
        canvas.fill_rounded_rect(bat_card, 16.0f, Color::Black);

        Rect bat_body(bat_card.x, bat_card.y, 228.0f, 83.0f);
        canvas.fill_rounded_rect(bat_body, 16.0f, Color::from_hex(0x0034FF));

        // 0.4s Battery ⇄ Lightning vector morph icon
        canvas.draw_morph_battery_lightning(Rect(bat_card.x + 40.0f, bat_card.y + 26.0f, 32.0f, 32.0f), battery_morph.val, Color::White);
        canvas.draw_text("99%", Point(bat_card.x + 130.0f, bat_card.y + 28.0f), 28.0f, Color::White, FontWeight::UltraBold, TextAlign::Center);
    }
}

bool Sidebar::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;

    if (mode == Mode::Devices) {
        Rect bat_card(bounds.x + 11.0f, bounds.y + 284.0f, 232.0f, 83.0f);
        if (bat_card.contains(x, y)) {
            battery_morph.set_target(battery_morph.target_val > 0.5f ? 0.0f : 1.0f);
            return true;
        }
    }

    if (mode == Mode::Library) {
        float row_y[4] = { 20.0f, 54.0f, 88.0f, 122.0f };
        for (int i = 0; i < 4; ++i) {
            Rect nrow(bounds.x, bounds.y + row_y[i], bounds.w, 32.0f);
            if (nrow.contains(x, y)) {
                if (i == 0 && !back_label.empty() && on_back) { on_back(); return true; }
                nav_selected = i;
                if (on_nav_change) on_nav_change(i);
                return true;
            }
        }

        // Playlist button: (4, 25, 92, 32) within new_cont (bounds.y + 156.0f)
        Rect pl_btn(bounds.x + 4.0f, bounds.y + 181.0f, 92.0f, 32.0f);
        if (pl_btn.contains(x, y)) {
            if (on_create_playlist_click) on_create_playlist_click();
            return true;
        }

        // Sparklist button: (100, 25, 92, 32) within new_cont
        Rect sp_btn(bounds.x + 100.0f, bounds.y + 181.0f, 92.0f, 32.0f);
        if (sp_btn.contains(x, y)) {
            if (on_create_sparklist_click) on_create_sparklist_click();
            return true;
        }

        // Playlist rows in Frame 6: (bounds.x + 2.0f, bounds.y + 230.0f)
        float pl_h = std::max(182.0f, bounds.h - 232.0f);
        Rect pl_cont(bounds.x + 2.0f, bounds.y + 230.0f, 192.0f, pl_h);
        int max_items = std::min(8, std::max(4, (int)((pl_cont.h - 22.0f) / 44.0f)));
        for (int i = 0; i < max_items; ++i) {
            float y_pos = 20.0f + i * 44.0f;
            Rect prow(pl_cont.x + 2.0f, pl_cont.y + y_pos, 176.0f, 41.0f);
            if (prow.contains(x, y)) {
                selected_playlist = i % 4;
                if (on_playlist_select) on_playlist_select(selected_playlist);
                return true;
            }
        }
    }
    return false;
}

void Sidebar::recalculate_storage() {
    // 256GB total capacity, 232.0f track width
    // Base storage if 8 items checked:
    // Rate 0: Lossless ~56GB
    // Rate 1: Opus 128 ~9GB
    // Rate 2: Opus 256 ~18GB
    // Rate 3: Opus 320 ~24GB
    static const float BASE_GB[4] = { 56.0f, 9.0f, 18.0f, 24.0f };
    float base = BASE_GB[std::max(0, std::min(3, current_rate))];
    float fraction = (float)current_sync_count / 8.0f;
    float used_gb = std::max(1.0f, base * fraction);
    float target_px = (used_gb / 256.0f) * 232.0f;
    storage_anim.set_target(target_px);

    int free_gb = (int)(256.0f - used_gb + 0.5f);
    int used_int = (int)(used_gb + 0.5f);
    storage_text = "Music 01 - 256GB - " + std::to_string(free_gb) + "GB Free (" + std::to_string(used_int) + "GB Used)";
}

void Sidebar::set_storage_rate(int rate) {
    current_rate = rate;
    recalculate_storage();
}

void Sidebar::set_sync_count(int count) {
    current_sync_count = count;
    recalculate_storage();
}

} // namespace MusicUI
