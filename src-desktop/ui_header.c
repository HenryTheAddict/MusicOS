#include "ui_header.h"
#include "draw_utils.h"

void ui_header_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    if (state->mode == MODE_LIBRARY) {
        // Library Subheader: Frame 5 (200, 64, 518x32), fill #d6edfdff
        draw_rect_filled(renderer, 200, 64, 518, 32, 0xD6EDFD);

        // Left label: 'Recently Added' at (8, 9, 106x14) -> (208, 73)
        draw_text(renderer, assets->fonts.font_13, "Recently Added", 208, 73, 0x000000);
        // tinyMinimize chevron at (116, 8, 16x16) -> (316, 72)
        draw_texture(renderer, assets->textures.ic_tiny_min, 316, 72, 16, 16);

        // Search box: Frame (262, 0, 256x32) -> (462, 64), fill #869fffff
        draw_rect_filled(renderer, 462, 64, 256, 32, 0x869FFF);
        draw_text(renderer, assets->fonts.font_20, "Search!", 462 + 84, 67, 0x38436B);
    } else {
        // Devices Subheader: (2, 65, 716x32), fill #d6edfdff
        draw_rect_filled(renderer, 2, 65, 716, 32, 0xD6EDFD);

        // Left label: "Henry Music 01"
        draw_text(renderer, assets->fonts.font_13, "Henry Music 01", 10, 73, 0x000000);
        draw_texture(renderer, assets->textures.ic_tiny_min, 115, 73, 16, 16);

        // 3 Option buttons at right
        for (int i = 0; i < 3; i++) {
            float bx = 716.0f - (3 - i) * 72.0f;
            draw_rounded_rect_filled(renderer, bx, 69, 64, 24, 4.0f, 0x0034FF);
            draw_text_in_box(renderer, assets->fonts.font_11, "Option", bx, 69, 64, 24, true, true, 0xFFFFFF);
        }
    }
}
