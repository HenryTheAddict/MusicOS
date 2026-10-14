#include "ui_toolbar.h"
#include "draw_utils.h"
#include <stdio.h>

void ui_toolbar_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets) {
    // 1. Top Bar Frame: (0, 0, 720x62), fill #00baffff
    draw_rect_filled(renderer, 0, 0, 720, 62, 0x00BAFF);

    // 2. Window Controls (Traffic Lights): directly on top bar (no pill background)
    // Close button: (18, 19, 24x24), fill #ff0000ff, border #c90000ff
    draw_circle_filled(renderer, 18 + 12, 19 + 12, 12.0f, 0xFF0000);
    draw_circle_outline(renderer, 18 + 12, 19 + 12, 12.0f, 1.0f, 0xC90000);
    draw_texture(renderer, assets->textures.ic_tiny_x, 26, 27, 8, 8);

    // Minimize button: (54, 19, 24x24), fill #ffe100ff, border #ffa900ff
    draw_circle_filled(renderer, 54 + 12, 19 + 12, 12.0f, 0xFFE100);
    draw_circle_outline(renderer, 54 + 12, 19 + 12, 12.0f, 1.0f, 0xFFA900);
    draw_texture(renderer, assets->textures.ic_tiny_min, 62, 27, 8, 8);

    // Maximize button: (90, 19, 24x24), fill #1fc800ff, border #1bab00ff
    draw_circle_filled(renderer, 90 + 12, 19 + 12, 12.0f, 0x1FC800);
    draw_circle_outline(renderer, 90 + 12, 19 + 12, 12.0f, 1.0f, 0x1BAB00);
    draw_texture(renderer, assets->textures.ic_tiny_max, 98, 27, 8, 8);

    // 3. Scope Pill: Frame 3 at (130, 7, 222x48), fill #0064dfff
    draw_pill_filled(renderer, 130, 7, 222, 48, 0x0064DF);

    // Pill title: Pure white #FFFFFF in Sketch
    const char* mode_title = (state->mode == MODE_DEVICES) ? "Devices" : "Library";
    draw_text(renderer, assets->fonts.font_20, mode_title, 150, 18, 0xFFFFFF);

    // Chevron downward V-line at (320, 27) -> (330, 34) <- (340, 27)
    SDL_SetRenderDrawColor(renderer, 255, 255, 255, 255);
    SDL_RenderLine(renderer, 320, 27, 330, 34);
    SDL_RenderLine(renderer, 320, 28, 330, 35);
    SDL_RenderLine(renderer, 330, 34, 340, 27);
    SDL_RenderLine(renderer, 330, 35, 340, 28);

    // 4. Now Playing Pill: Frame 2 at (368, 7, 334x48), fill #FFFFFF
    draw_pill_filled(renderer, 368, 7, 334, 48, 0xFFFFFF);

    // Artwork: (8, 8, 32x32) relative -> (376, 15)
    draw_rounded_rect_filled(renderer, 376, 15, 32, 32, 4.0f, 0xD6DEFD);
    draw_texture(renderer, assets->textures.cv_sky, 376, 15, 32, 32);

    // Track Title: (44, 7, 145x12) relative -> (412, 13.5f)
    draw_text(renderer, assets->fonts.font_11, state->now_playing.title, 412, 13.5f, 0x000000);

    // Artist: (44, 22, 114x8) relative -> (412, 27.5f)
    draw_text(renderer, assets->fonts.font_8, state->now_playing.artist, 412, 27.5f, 0x707070);

    // Progress bar: (44, 34, 145x4) relative -> (412, 41, 145x4)
    draw_rounded_rect_filled(renderer, 412, 41, 145, 4, 2.0f, 0xD6EFFD);
    float progress_w = 145.0f * state->now_playing.progress;
    if (progress_w > 145.0f) progress_w = 145.0f;
    if (progress_w > 0.0f) {
        draw_rounded_rect_filled(renderer, 412, 41, progress_w, 4, 2.0f, 0x00A6FF);
    }

    // Queue icon: (201, 8, 16x16) relative -> (569, 15)
    draw_texture(renderer, assets->textures.ic_queue, 569, 15, 16, 16);
    // Lossless badge: (217, 8, 16x16) relative -> (585, 15)
    draw_texture(renderer, assets->textures.ic_lossless, 585, 15, 16, 16);

    // Repeat badge: (201, 24, 16x16) relative -> (569, 31)
    SDL_Texture* rep_tex = state->now_playing.repeat_on ? assets->textures.ic_repeat_on : assets->textures.ic_repeat_off;
    draw_texture(renderer, rep_tex, 569, 31, 16, 16);

    // Shuffle badge: (217, 24, 16x16) relative -> (585, 31)
    SDL_Texture* shuf_tex = state->now_playing.shuffle_on ? assets->textures.ic_shuffle_on : assets->textures.ic_shuffle_off;
    draw_texture(renderer, shuf_tex, 585, 31, 16, 16);

    // Transport buttons: (233, 8, 32x32) relative -> (601, 15)
    draw_texture(renderer, assets->textures.ic_prev, 601, 15, 32, 32);
    // Pause button: (265, 8, 32x32) relative -> (633, 15)
    SDL_Texture* play_tex = state->now_playing.is_playing ? assets->textures.ic_pause : assets->textures.ic_play;
    draw_texture(renderer, play_tex, 633, 15, 32, 32);
    // Next button: (297, 8, 32x32) relative -> (665, 15)
    draw_texture(renderer, assets->textures.ic_next, 665, 15, 32, 32);
}

bool ui_toolbar_handle_click(AppState* state, float x, float y, bool* should_quit, bool* should_minimize) {
    if (x >= 18 && x <= 114 && y >= 19 && y <= 43) {
        if (x >= 18 && x <= 42) {
            *should_quit = true;
            return true;
        }
        if (x >= 54 && x <= 78) {
            *should_minimize = true;
            return true;
        }
        if (x >= 90 && x <= 114) {
            return true;
        }
    }

    // Check scope dropdown button: (130, 7, 222x48)
    if (x >= 130 && x <= 352 && y >= 7 && y <= 55) {
        state->mode = (state->mode == MODE_LIBRARY) ? MODE_DEVICES : MODE_LIBRARY;
        return true;
    }

    // Check Now Playing controls: (368, 7, 334x48)
    if (x >= 368 && x <= 702 && y >= 7 && y <= 55) {
        // Repeat: (569, 31, 16, 16)
        if (x >= 569 && x <= 585 && y >= 31 && y <= 47) {
            app_state_toggle_repeat(state);
            return true;
        }
        // Shuffle: (585, 31, 16, 16)
        if (x >= 585 && x <= 601 && y >= 31 && y <= 47) {
            app_state_toggle_shuffle(state);
            return true;
        }
        // Play/Pause: (633, 15, 32, 32)
        if (x >= 633 && x <= 665 && y >= 15 && y <= 47) {
            app_state_toggle_playback(state);
            return true;
        }
    }

    return false;
}
