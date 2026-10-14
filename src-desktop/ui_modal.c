#include "ui_modal.h"
#include "draw_utils.h"

void ui_modal_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    if (!state->sparklist_modal_open) return;

    // Frame 8: (232, 30, 256, 420), gradient #0063FF -> #0037FF, radius 16px
    float mx = 232.0f;
    float my = 30.0f;

    // Drop shadow under modal
    draw_rounded_rect_filled_rgba(renderer, mx, my + 4, 256, 420, 16.0f, 0, 0, 0, 60);

    draw_rounded_rect_vertical_gradient(renderer, mx, my, 256, 420, 16.0f, 0x0063FF, 0x0037FF);
    draw_rounded_rect_outline(renderer, mx, my, 256, 420, 16.0f, 3.0f, 0xFFFFFF);

    // Sparklist Icon on white circle: (112, 19, 32, 32) relative -> (mx + 112, my + 19)
    draw_circle_filled(renderer, mx + 112 + 16, my + 19 + 16, 16.0f, 0xFFFFFF);
    draw_texture(renderer, assets->textures.ic_sparklist, mx + 112, my + 19, 32, 32);

    // 4 Title lines: centered at mx + 128 (360)
    // "Hello!\nChoose a song\nyou like the\nvibe of!"
    float cx = mx + 128.0f;
    draw_text_centered(renderer, assets->fonts.font_20, "Hello!", cx, 86 + 6, 0xFFFFFF);
    draw_text_centered(renderer, assets->fonts.font_20, "Choose a song", cx, 114 + 6, 0xFFFFFF);
    draw_text_centered(renderer, assets->fonts.font_20, "you like the", cx, 142 + 6, 0xFFFFFF);
    draw_text_centered(renderer, assets->fonts.font_20, "vibe of!", cx, 170 + 6, 0xFFFFFF);

    // Lower Frame: (16, 182, 225, 139) -> (mx + 16, my + 182) = (248, 212), fill #D6DEFD
    draw_rounded_rect_filled(renderer, mx + 16, my + 182, 225, 139, 4.0f, 0xD6DEFD);
}

bool ui_modal_handle_click(AppState* state, float x, float y) {
    if (!state->sparklist_modal_open) return false;

    float mx = 232.0f;
    float my = 30.0f;

    // Click outside modal card closes it
    if (x < mx || x > mx + 256 || y < my || y > my + 420) {
        state->sparklist_modal_open = false;
        return true;
    }

    return true; // Modal intercepts all clicks inside it
}
