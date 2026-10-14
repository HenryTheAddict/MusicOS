#include "ui_content.h"
#include "draw_utils.h"

void ui_content_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    if (state->mode == MODE_LIBRARY) {
        // Content Box: Frame 7 (200, 100, 515x378), fill #00aeffff
        draw_rect_filled(renderer, 200, 100, 515, 378, 0x00AEFF);

        // 3-column x 2-row album grid (Group 21, 21, 448x336)
        float col_offsets[3] = { 0.0f, 170.0f, 340.0f };
        float row_offsets[2] = { 0.0f, 176.0f };

        for (int i = 0; i < 6; i++) {
            int col = i % 3;
            int row = i / 3;
            float cx = 221.0f + col_offsets[col];
            float cy = 121.0f + row_offsets[row];

            // 128x128 Artwork Frame, fill #000000ff
            draw_rect_filled(renderer, cx, cy, 128, 128, 0x000000);

            // Title: (19, 134, 89x14) relative
            draw_text(renderer, assets->fonts.font_13, state->albums[i].title, cx + 19, cy + 134, 0x000000);

            // Artist: (51, 152, 24x8) relative
            draw_text(renderer, assets->fonts.font_8, state->albums[i].artist, cx + 51, cy + 152, 0x000000);
        }

        // Scrollbar: Frame 2 (504, 61, 8x256) relative -> (704, 161, 8x256), fill #0034ffff
        draw_rect_filled(renderer, 704, 161, 8, 256, 0x0034FF);
        // Thumb: (1, 1, 6x32) relative -> (705, 162), fill #ffffffff
        draw_rect_filled(renderer, 705, 162, 6, 32, 0xFFFFFF);
    } else {
        // Devices Content Box: (259, 100, 456x378), fill #00aeffff
        draw_rect_filled(renderer, 259, 100, 456, 378, 0x00AEFF);

        // Centered text: "choosing what to sync\nis coming soon."
        draw_text_centered(renderer, assets->fonts.font_20, "choosing what to sync", 259 + 228, 100 + 175, 0x000000);
        draw_text_centered(renderer, assets->fonts.font_20, "is coming soon.", 259 + 228, 100 + 215, 0x000000);
    }
}

bool ui_content_handle_click(AppState* state, float x, float y) {
    if (state->mode == MODE_LIBRARY) {
        float col_offsets[3] = { 0.0f, 170.0f, 340.0f };
        float row_offsets[2] = { 0.0f, 176.0f };

        for (int i = 0; i < 6; i++) {
            int col = i % 3;
            int row = i / 3;
            float cx = 221.0f + col_offsets[col];
            float cy = 121.0f + row_offsets[row];
            if (x >= cx && x <= cx + 128 && y >= cy && y <= cy + 160) {
                state->selected_album = i;
                return true;
            }
        }
    }
    return false;
}
