#include "ui_sidebar.h"
#include "draw_utils.h"
#include <stdio.h>

static void render_standard_sidebar(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    // Frame 4: (2, 64, 196, 414), fill #cbf1ffff
    draw_rect_filled(renderer, 2, 64, 196, 414, 0xCBF1FF);

    // Header: Frame (0, 0, 196x20) -> (2, 64), fill #D6DEFD
    draw_rect_filled(renderer, 2, 64, 196, 20, 0xD6DEFD);
    draw_text(renderer, assets->fonts.font_11, "Library", 78, 67, 0x000000);

    // Sidebar Items: Albums (y=84), Artists (y=118), Genres (y=152), Songs (y=186)
    const char* titles[4] = { "Albums", "Artists", "Genres", "Songs" };
    SDL_Texture* icons[4] = { assets->textures.ic_album, assets->textures.ic_artist, assets->textures.ic_genre, assets->textures.ic_song };
    float item_ys[4] = { 84.0f, 118.0f, 152.0f, 186.0f };
    float text_xs[4] = { 74.0f, 80.0f, 76.0f, 79.0f };

    for (int i = 0; i < 4; i++) {
        float y = item_ys[i];
        bool is_selected = (state->mode == MODE_LIBRARY && (int)state->section == i);

        if (is_selected) {
            // Selected item: (0, 20, 196x32) -> (2, 84), fill #009affff
            draw_rect_filled(renderer, 2, y, 196, 32, 0x009AFF);
            draw_texture(renderer, icons[i], 2, y, 32, 32);
            draw_text(renderer, assets->fonts.font_13, titles[i], text_xs[i], y + 9, 0x002B48);
            draw_texture(renderer, assets->textures.ic_3dots, 166, y, 32, 32);
        } else {
            // Unselected items: fill #AAE4FF
            draw_rect_filled(renderer, 2, y, 196, 32, 0xAAE4FF);
            draw_texture(renderer, icons[i], 2, y, 32, 32);
            draw_text(renderer, assets->fonts.font_13, titles[i], text_xs[i], y + 9, 0x000000);
        }
    }

    // "New:" container: Frame 2 (0, 156, 196x64) -> (2, 220), fill #00baff80 on #cbf1ff => #65D5FF
    draw_rect_filled(renderer, 2, 220, 196, 64, 0x65D5FF);
    draw_text(renderer, assets->fonts.font_13, "New:", 2 + 80, 220 + 7, 0x000000);

    // Playlist button: Frame 3 (4, 25, 92x32) -> (6, 245), fill #00e6ffff
    draw_rounded_rect_filled(renderer, 6, 245, 92, 32, 4.0f, 0x00E6FF);
    draw_rounded_rect_outline(renderer, 6, 245, 92, 32, 4.0f, 1.0f, 0x326A7F);
    draw_texture(renderer, assets->textures.ic_playlist, 6, 245, 32, 32);
    draw_text(renderer, assets->fonts.font_13, "Playlist", 36, 255.0f, 0x000000);

    // Sparklist button: Frame 3 (100, 25, 92x32) -> (102, 245), gradient fill #00E4FF to #5073FF
    draw_rounded_rect_vertical_gradient(renderer, 102, 245, 92, 32, 4.0f, 0x00E4FF, 0x5073FF);
    draw_rounded_rect_outline(renderer, 102, 245, 92, 32, 4.0f, 1.0f, 0x326A7F);
    draw_texture(renderer, assets->textures.ic_sparklist, 102, 245, 32, 32);
    draw_text(renderer, assets->fonts.font_13, "Sparklist", 134, 255.0f, 0x000000);

    // Blue divider line: Frame 4 (0, 224, 196x2) -> (2, 288), fill #0075ffff
    draw_rect_filled(renderer, 2, 288, 196, 2, 0x0075FF);

    // Playlists Box ("{user}s stuff:"): Frame 6 (2, 229, 192x184) -> (4, 293), fill #54b9ffff
    draw_rounded_rect_filled(renderer, 4, 293, 192, 184, 4.0f, 0x54B9FF);

    // Header: Frame 3 (0, 0, 192x20) -> (4, 293), smooth gradient from #27F1FD to #54B9FF with rounded top
    draw_rounded_top_rect_vertical_gradient(renderer, 4, 293, 192, 20, 4.0f, 0x27F1FD, 0x54B9FF);
    draw_text(renderer, assets->fonts.font_11, "{user}s stuff:", 4 + 57, 293 + 4, 0x000000);

    // 4 Playlist items: fill #9BCDF0
    SDL_Texture* cv_list[4] = { assets->textures.cv_overcast, assets->textures.cv_tritower, assets->textures.cv_sparklist_1, assets->textures.cv_overcast };
    float item_y[4] = { 314.0f, 357.0f, 400.0f, 444.0f };
    float item_x[4] = { 6.0f, 6.0f, 6.0f, 7.0f };

    for (int i = 0; i < 4; i++) {
        float px = item_x[i];
        float py = item_y[i];
        float ph = (i == 3) ? 33.0f : 41.0f; // Item 3 clipped by Frame 6 bottom border
        draw_rounded_rect_filled(renderer, px, py, 176, ph, 4.0f, 0x9BCDF0);

        // 32x32 Cover at (4, 5) relative -> (px + 4, py + 5)
        float ch = (i == 3) ? 27.0f : 32.0f;
        draw_texture(renderer, cv_list[i], px + 4, py + 5, 32, ch);

        // Title at (38, 5) -> (px + 38, py + 6.5)
        draw_text(renderer, assets->fonts.font_11, state->playlists[i].title, px + 38, py + 6.5f, 0x000000);

        // Blue Underline at (39, 21, 99x2) -> (px + 39, py + 21)
        draw_rect_filled(renderer, px + 39, py + 21, 99, 2, 0x0041FF);

        // Subtitle at (39, 26) -> (px + 39, py + 21)
        if (i < 3) {
            draw_text(renderer, assets->fonts.font_8, state->playlists[i].subtitle, px + 39, py + 21.0f, 0x404040);
        }

        if (state->playlists[i].is_sparklist) {
            // Sparklist icon: (97, 4, 16x16) -> (px + 97, py + 4)
            draw_texture(renderer, assets->textures.ic_sparklist, px + 97, py + 4, 16, 16);
        }

        // ShufflePlay button: (140, 5, 32x32) -> (px + 140, py + 5)
        float sph = (i == 3) ? 27.0f : 32.0f;
        draw_texture(renderer, assets->textures.ic_shuffle_play, px + 140, py + 5, 32, sph);
    }

    // Scrollbar: Frame (181, 17, 8x156) -> (185, 310), fill #739fffff
    draw_rect_filled(renderer, 185, 310, 8, 158, 0x739FFF);
    // Thumb: (0, 0, 8x33) -> fill #0048ffff
    draw_rect_filled(renderer, 185, 310, 8, 33, 0x0048FF);
    draw_rect_outline(renderer, 185, 310, 8, 158, 1.0f, 0x00195E);

    // Playlists Box border outline on top:
    draw_rounded_rect_outline(renderer, 4, 293, 192, 184, 4.0f, 1.0f, 0x0041FF);
}

static void render_devices_sidebar(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    (void)state;
    // Devices Mode Sidebar: Frame 4 at (2, 100, 255, 378), fill #CBF1FF
    draw_rect_filled(renderer, 2, 100, 255, 378, 0xCBF1FF);

    // Device summary row at y = 110
    draw_texture(renderer, assets->textures.ic_device, 13, 110, 32, 32);
    draw_text(renderer, assets->fonts.font_13, "Henry Music 01", 45, 110, 0x000000);
    draw_text(renderer, assets->fonts.font_8, "Music 01 - 256GB - 200GB Free", 45, 130, 0x606060);
    draw_texture(renderer, assets->textures.ic_3dots_incircle, 173, 110, 16, 16);

    // Card 1: Storage Bar Card at (13, 150, 232, 42), fill #00C3FF
    draw_rounded_rect_filled(renderer, 13, 150, 232, 42, 4.0f, 0x00C3FF);
    draw_rounded_rect_filled(renderer, 13, 150, 232, 24, 4.0f, 0x00E5FF);
    draw_rounded_rect_filled(renderer, 13, 150, 63, 24, 4.0f, 0x0064DF);
    draw_text(renderer, assets->fonts.font_11, "Music", 18, 176, 0x000000);
    draw_text(renderer, assets->fonts.font_11, "Free", 205, 175, 0x000000);

    // Card 2: Quick(er) Settings at (13, 203, 232, 103), fill #81C7FF
    draw_rounded_rect_filled(renderer, 13, 203, 232, 103, 4.0f, 0x81C7FF);
    draw_text(renderer, assets->fonts.font_11, "Quick(er) Settings", 21, 210, 0x000000);

    // Lossless button at (27, 230, 96, 32)
    draw_rounded_rect_filled(renderer, 27, 230, 96, 32, 4.0f, 0x476CFF);
    draw_rounded_rect_outline(renderer, 27, 230, 96, 32, 4.0f, 1.0f, 0x003CFF);
    draw_texture(renderer, assets->textures.ic_lossless, 27, 230, 32, 32);
    draw_text(renderer, assets->fonts.font_11, "Lossless", 59, 239, 0xFFFFFF);

    // Internet button at (135, 230, 96, 32)
    draw_rounded_rect_filled(renderer, 135, 230, 96, 32, 4.0f, 0x476CFF);
    draw_rounded_rect_outline(renderer, 135, 230, 96, 32, 4.0f, 1.0f, 0x003CFF);
    draw_texture(renderer, assets->textures.ic_netsearch, 135, 230, 32, 32);
    draw_text(renderer, assets->fonts.font_11, "Internet", 167, 239, 0xFFFFFF);

    // Auto-Sync button at (27, 270, 96, 32)
    draw_rounded_rect_filled(renderer, 27, 270, 96, 32, 4.0f, 0xD2DBFF);
    draw_rounded_rect_outline(renderer, 27, 270, 96, 32, 4.0f, 1.0f, 0xFFFFFF);
    draw_texture(renderer, assets->textures.ic_file, 27, 270, 32, 32);
    draw_text(renderer, assets->fonts.font_11, "Auto-Sync", 59, 281, 0x000000);

    // Sparklist AutoUpdate button at (135, 270, 96, 32)
    draw_rounded_rect_filled(renderer, 135, 270, 96, 32, 4.0f, 0xD2DBFF);
    draw_rounded_rect_outline(renderer, 135, 270, 96, 32, 4.0f, 1.0f, 0xFFFFFF);
    draw_texture(renderer, assets->textures.ic_sparklist, 135, 270, 32, 32);
    draw_text(renderer, assets->fonts.font_8, "Sparklist", 167, 274, 0x000000);
    draw_text(renderer, assets->fonts.font_8, "AutoUpdate", 167, 285, 0x000000);

    // Card 3: Audio Settings at (13, 310, 232, 70), fill #81C7FF
    draw_rounded_rect_filled(renderer, 13, 310, 232, 70, 4.0f, 0x81C7FF);
    draw_text(renderer, assets->fonts.font_11, "Audio Settings", 77, 314, 0x000000);

    // Quality button at (29, 332, 96, 44)
    draw_rounded_rect_filled(renderer, 29, 332, 96, 44, 4.0f, 0x00ACFF);
    draw_rounded_rect_outline(renderer, 29, 332, 96, 44, 4.0f, 1.0f, 0x5F80FF);
    draw_text_in_box(renderer, assets->fonts.font_11, "Quality", 29, 332, 96, 44, true, true, 0x000000);

    // EQ button at (133, 332, 96, 44)
    draw_rounded_rect_filled(renderer, 133, 332, 96, 44, 4.0f, 0x5F80FF);
    draw_rounded_rect_outline(renderer, 133, 332, 96, 44, 4.0f, 1.0f, 0x00ACFF);
    draw_text_in_box(renderer, assets->fonts.font_11, "EQ", 133, 332, 96, 44, true, true, 0x000000);

    // Card 4: Battery Card at (13, 384, 232, 83), fill #000000
    draw_rounded_rect_filled(renderer, 13, 384, 232, 83, 4.0f, 0x000000);
    draw_rounded_rect_filled(renderer, 13, 384, 228, 83, 4.0f, 0x0034FF);
    draw_text(renderer, assets->fonts.font_20, "99%", 94, 412, 0xFFFFFF);

    // Separator line to content: x = 257
    draw_rect_filled(renderer, 257, 100, 2, 378, 0x0075FF);
}

void ui_sidebar_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    if (state->mode == MODE_DEVICES) {
        render_devices_sidebar(renderer, state, assets);
    } else {
        render_standard_sidebar(renderer, state, assets);
    }
}

bool ui_sidebar_handle_click(AppState* state, float x, float y) {
    if (state->mode == MODE_DEVICES) {
        return false;
    }

    // Check scope items
    if (x >= 2 && x <= 198) {
        if (y >= 84 && y < 118) { state->section = SECTION_ALBUMS; return true; }
        if (y >= 118 && y < 152) { state->section = SECTION_ARTISTS; return true; }
        if (y >= 152 && y < 186) { state->section = SECTION_GENRES; return true; }
        if (y >= 186 && y < 220) { state->section = SECTION_SONGS; return true; }

        // Playlist button: (6, 245, 92, 32)
        if (x >= 6 && x <= 98 && y >= 245 && y <= 277) {
            return true;
        }

        // Sparklist button: (102, 245, 92, 32)
        if (x >= 102 && x <= 194 && y >= 245 && y <= 277) {
            state->sparklist_modal_open = true;
            return true;
        }
    }

    return false;
}
