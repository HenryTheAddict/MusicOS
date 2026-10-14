#include "draw_utils.h"
#include <math.h>
#include <stdlib.h>

static SDL_Color hex_to_sdl_color(uint32_t hex, Uint8 alpha) {
    SDL_Color c;
    c.r = (Uint8)((hex >> 16) & 0xFF);
    c.g = (Uint8)((hex >> 8) & 0xFF);
    c.b = (Uint8)(hex & 0xFF);
    c.a = alpha;
    return c;
}

static void draw_set_color_hex(SDL_Renderer* renderer, uint32_t hex) {
    SDL_Color c = hex_to_sdl_color(hex, 255);
    SDL_SetRenderDrawColor(renderer, c.r, c.g, c.b, c.a);
}

void draw_rect_filled(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t hex) {
    draw_set_color_hex(renderer, hex);
    SDL_FRect r = { x, y, w, h };
    SDL_RenderFillRect(renderer, &r);
}

void draw_rect_filled_rgba(SDL_Renderer* renderer, float x, float y, float w, float h, Uint8 r, Uint8 g, Uint8 b, Uint8 a) {
    SDL_SetRenderDrawColor(renderer, r, g, b, a);
    SDL_FRect rect = { x, y, w, h };
    SDL_RenderFillRect(renderer, &rect);
}

void draw_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t top_hex, uint32_t bot_hex) {
    SDL_Color top_c = hex_to_sdl_color(top_hex, 255);
    SDL_Color bot_c = hex_to_sdl_color(bot_hex, 255);

    int steps = (int)ceilf(h);
    for (int i = 0; i < steps; i++) {
        float cur_y = y + (float)i;
        float t = (float)i / (float)(steps > 1 ? (steps - 1) : 1);
        Uint8 r = (Uint8)(top_c.r + t * (bot_c.r - top_c.r));
        Uint8 g = (Uint8)(top_c.g + t * (bot_c.g - top_c.g));
        Uint8 b = (Uint8)(top_c.b + t * (bot_c.b - top_c.b));
        SDL_SetRenderDrawColor(renderer, r, g, b, 255);
        SDL_FRect line = { x, cur_y, w, 1.0f };
        SDL_RenderFillRect(renderer, &line);
    }
}

void draw_rounded_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t top_hex, uint32_t bot_hex) {
    if (radius <= 0.0f) {
        draw_rect_vertical_gradient(renderer, x, y, w, h, top_hex, bot_hex);
        return;
    }
    if (radius > w / 2.0f) radius = w / 2.0f;
    if (radius > h / 2.0f) radius = h / 2.0f;

    SDL_Color top_c = hex_to_sdl_color(top_hex, 255);
    SDL_Color bot_c = hex_to_sdl_color(bot_hex, 255);
    int steps = (int)ceilf(h);

    for (int i = 0; i < steps; i++) {
        float cur_y = y + (float)i;
        float t = (float)i / (float)(steps > 1 ? (steps - 1) : 1);
        Uint8 r = (Uint8)(top_c.r + t * (bot_c.r - top_c.r));
        Uint8 g = (Uint8)(top_c.g + t * (bot_c.g - top_c.g));
        Uint8 b = (Uint8)(top_c.b + t * (bot_c.b - top_c.b));
        SDL_SetRenderDrawColor(renderer, r, g, b, 255);

        float line_x = x;
        float line_w = w;

        // Top corner curve
        if (i < radius) {
            float dy = radius - (float)i;
            float dx = radius - sqrtf(fmaxf(0.0f, radius * radius - dy * dy));
            line_x = x + dx;
            line_w = w - 2.0f * dx;
        }
        // Bottom corner curve
        else if (i >= steps - radius) {
            float dy = (float)i - (steps - radius);
            float dx = radius - sqrtf(fmaxf(0.0f, radius * radius - dy * dy));
            line_x = x + dx;
            line_w = w - 2.0f * dx;
        }

        SDL_FRect line = { line_x, cur_y, line_w, 1.0f };
        SDL_RenderFillRect(renderer, &line);
    }
}

void draw_rounded_top_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t top_hex, uint32_t bot_hex) {
    if (radius <= 0.0f) {
        draw_rect_vertical_gradient(renderer, x, y, w, h, top_hex, bot_hex);
        return;
    }
    if (radius > w / 2.0f) radius = w / 2.0f;
    if (radius > h / 2.0f) radius = h / 2.0f;

    SDL_Color top_c = hex_to_sdl_color(top_hex, 255);
    SDL_Color bot_c = hex_to_sdl_color(bot_hex, 255);
    int steps = (int)ceilf(h);

    for (int i = 0; i < steps; i++) {
        float cur_y = y + (float)i;
        float t = (float)i / (float)(steps > 1 ? (steps - 1) : 1);
        Uint8 r = (Uint8)(top_c.r + t * (bot_c.r - top_c.r));
        Uint8 g = (Uint8)(top_c.g + t * (bot_c.g - top_c.g));
        Uint8 b = (Uint8)(top_c.b + t * (bot_c.b - top_c.b));
        SDL_SetRenderDrawColor(renderer, r, g, b, 255);

        float line_x = x;
        float line_w = w;

        // Only Top corner curve
        if (i < radius) {
            float dy = radius - (float)i;
            float dx = radius - sqrtf(fmaxf(0.0f, radius * radius - dy * dy));
            line_x = x + dx;
            line_w = w - 2.0f * dx;
        }

        SDL_FRect line = { line_x, cur_y, line_w, 1.0f };
        SDL_RenderFillRect(renderer, &line);
    }
}

void draw_rect_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float thickness, uint32_t hex) {
    draw_set_color_hex(renderer, hex);
    SDL_FRect top = { x, y, w, thickness };
    SDL_FRect bottom = { x, y + h - thickness, w, thickness };
    SDL_FRect left = { x, y, thickness, h };
    SDL_FRect right = { x + w - thickness, y, thickness, h };
    SDL_RenderFillRect(renderer, &top);
    SDL_RenderFillRect(renderer, &bottom);
    SDL_RenderFillRect(renderer, &left);
    SDL_RenderFillRect(renderer, &right);
}

void draw_circle_filled(SDL_Renderer* renderer, float cx, float cy, float radius, uint32_t hex) {
    draw_set_color_hex(renderer, hex);
    int r = (int)ceilf(radius);
    for (int dy = -r; dy <= r; dy++) {
        float dx = sqrtf(fmaxf(0.0f, radius * radius - (float)(dy * dy)));
        SDL_FRect line = { cx - dx, cy + (float)dy, 2.0f * dx, 1.0f };
        SDL_RenderFillRect(renderer, &line);
    }
}

void draw_circle_outline(SDL_Renderer* renderer, float cx, float cy, float radius, float thickness, uint32_t hex) {
    draw_set_color_hex(renderer, hex);
    float r_outer = radius;
    float r_inner = radius - thickness;
    int r = (int)ceilf(r_outer);
    for (int dy = -r; dy <= r; dy++) {
        float dx_out = sqrtf(fmaxf(0.0f, r_outer * r_outer - (float)(dy * dy)));
        float dx_in = (fabsf((float)dy) < r_inner) ? sqrtf(fmaxf(0.0f, r_inner * r_inner - (float)(dy * dy))) : 0.0f;
        if (dx_in <= 0.0f) {
            SDL_FRect line = { cx - dx_out, cy + (float)dy, 2.0f * dx_out, 1.0f };
            SDL_RenderFillRect(renderer, &line);
        } else {
            SDL_FRect l1 = { cx - dx_out, cy + (float)dy, dx_out - dx_in, 1.0f };
            SDL_FRect l2 = { cx + dx_in, cy + (float)dy, dx_out - dx_in, 1.0f };
            SDL_RenderFillRect(renderer, &l1);
            SDL_RenderFillRect(renderer, &l2);
        }
    }
}

void draw_rounded_rect_filled(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t hex) {
    if (radius <= 0.0f) {
        draw_rect_filled(renderer, x, y, w, h, hex);
        return;
    }
    if (radius > w / 2.0f) radius = w / 2.0f;
    if (radius > h / 2.0f) radius = h / 2.0f;

    draw_set_color_hex(renderer, hex);
    int r = (int)ceilf(radius);

    SDL_FRect center = { x, y + radius, w, h - 2.0f * radius };
    SDL_RenderFillRect(renderer, &center);

    for (int dy = 0; dy < r; dy++) {
        float fdy = radius - (float)dy;
        float dx = radius - sqrtf(fmaxf(0.0f, radius * radius - fdy * fdy));
        float line_w = w - 2.0f * dx;

        SDL_FRect top_line = { x + dx, y + (float)dy, line_w, 1.0f };
        SDL_FRect bot_line = { x + dx, y + h - 1.0f - (float)dy, line_w, 1.0f };
        SDL_RenderFillRect(renderer, &top_line);
        SDL_RenderFillRect(renderer, &bot_line);
    }
}

void draw_rounded_rect_filled_rgba(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, Uint8 r_col, Uint8 g_col, Uint8 b_col, Uint8 a_col) {
    if (radius <= 0.0f) {
        draw_rect_filled_rgba(renderer, x, y, w, h, r_col, g_col, b_col, a_col);
        return;
    }
    if (radius > w / 2.0f) radius = w / 2.0f;
    if (radius > h / 2.0f) radius = h / 2.0f;

    SDL_SetRenderDrawBlendMode(renderer, SDL_BLENDMODE_BLEND);
    SDL_SetRenderDrawColor(renderer, r_col, g_col, b_col, a_col);
    int r = (int)ceilf(radius);

    SDL_FRect center = { x, y + radius, w, h - 2.0f * radius };
    SDL_RenderFillRect(renderer, &center);

    for (int dy = 0; dy < r; dy++) {
        float fdy = radius - (float)dy;
        float dx = radius - sqrtf(fmaxf(0.0f, radius * radius - fdy * fdy));
        float line_w = w - 2.0f * dx;

        SDL_FRect top_line = { x + dx, y + (float)dy, line_w, 1.0f };
        SDL_FRect bot_line = { x + dx, y + h - 1.0f - (float)dy, line_w, 1.0f };
        SDL_RenderFillRect(renderer, &top_line);
        SDL_RenderFillRect(renderer, &bot_line);
    }
    SDL_SetRenderDrawBlendMode(renderer, SDL_BLENDMODE_NONE);
}

void draw_rounded_rect_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, float thickness, uint32_t hex) {
    draw_set_color_hex(renderer, hex);
    if (radius <= 0.0f) {
        draw_rect_outline(renderer, x, y, w, h, thickness, hex);
        return;
    }

    SDL_FRect top = { x + radius, y, w - 2.0f * radius, thickness };
    SDL_FRect bottom = { x + radius, y + h - thickness, w - 2.0f * radius, thickness };
    SDL_FRect left = { x, y + radius, thickness, h - 2.0f * radius };
    SDL_FRect right = { x + w - thickness, y + radius, thickness, h - 2.0f * radius };
    SDL_RenderFillRect(renderer, &top);
    SDL_RenderFillRect(renderer, &bottom);
    SDL_RenderFillRect(renderer, &left);
    SDL_RenderFillRect(renderer, &right);

    // 4 Corner arcs
    float centers[4][2] = {
        { x + radius, y + radius },
        { x + w - radius, y + radius },
        { x + radius, y + h - radius },
        { x + w - radius, y + h - radius }
    };
    int quadrant_signs[4][2] = {
        { -1, -1 }, { 1, -1 }, { -1, 1 }, { 1, 1 }
    };

    float r_out = radius;
    float r_in = radius - thickness;
    int steps = (int)ceilf(radius);

    for (int c = 0; c < 4; c++) {
        float cx = centers[c][0];
        float cy = centers[c][1];
        int sx = quadrant_signs[c][0];
        int sy = quadrant_signs[c][1];

        for (int i = 0; i < steps; i++) {
            float dy = (float)i;
            float dx_out = sqrtf(fmaxf(0.0f, r_out * r_out - dy * dy));
            float dx_in = (dy < r_in) ? sqrtf(fmaxf(0.0f, r_in * r_in - dy * dy)) : 0.0f;
            float line_w = dx_out - dx_in;

            float py = cy + (float)sy * dy;
            float px = (sx > 0) ? (cx + dx_in) : (cx - dx_out);
            SDL_FRect arc_pixel = { px, py, line_w, 1.0f };
            SDL_RenderFillRect(renderer, &arc_pixel);
        }
    }
}

void draw_pill_filled(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t hex) {
    draw_rounded_rect_filled(renderer, x, y, w, h, h / 2.0f, hex);
}

void draw_pill_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float thickness, uint32_t hex) {
    draw_rounded_rect_outline(renderer, x, y, w, h, h / 2.0f, thickness, hex);
}

void draw_text(SDL_Renderer* renderer, TTF_Font* font, const char* text, float x, float y, uint32_t hex) {
    if (!font || !text || !*text) return;
    SDL_Color col = hex_to_sdl_color(hex, 255);
    SDL_Surface* surf = TTF_RenderText_Blended(font, text, 0, col);
    if (!surf) return;
    SDL_Texture* tex = SDL_CreateTextureFromSurface(renderer, surf);
    if (tex) {
        SDL_FRect dst = { x, y, (float)surf->w, (float)surf->h };
        SDL_RenderTexture(renderer, tex, NULL, &dst);
        SDL_DestroyTexture(tex);
    }
    SDL_DestroySurface(surf);
}

void draw_text_centered(SDL_Renderer* renderer, TTF_Font* font, const char* text, float cx, float cy, uint32_t hex) {
    if (!font || !text || !*text) return;
    SDL_Color col = hex_to_sdl_color(hex, 255);
    SDL_Surface* surf = TTF_RenderText_Blended(font, text, 0, col);
    if (!surf) return;
    SDL_Texture* tex = SDL_CreateTextureFromSurface(renderer, surf);
    if (tex) {
        SDL_FRect dst = { cx - (float)surf->w / 2.0f, cy - (float)surf->h / 2.0f, (float)surf->w, (float)surf->h };
        SDL_RenderTexture(renderer, tex, NULL, &dst);
        SDL_DestroyTexture(tex);
    }
    SDL_DestroySurface(surf);
}

void draw_text_in_box(SDL_Renderer* renderer, TTF_Font* font, const char* text, float x, float y, float w, float h, bool center_x, bool center_y, uint32_t hex) {
    if (!font || !text || !*text) return;
    SDL_Color col = hex_to_sdl_color(hex, 255);
    SDL_Surface* surf = TTF_RenderText_Blended(font, text, 0, col);
    if (!surf) return;
    SDL_Texture* tex = SDL_CreateTextureFromSurface(renderer, surf);
    if (tex) {
        float tx = center_x ? (x + (w - (float)surf->w) / 2.0f) : x;
        float ty = center_y ? (y + (h - (float)surf->h) / 2.0f) : y;
        SDL_FRect dst = { tx, ty, (float)surf->w, (float)surf->h };
        SDL_RenderTexture(renderer, tex, NULL, &dst);
        SDL_DestroyTexture(tex);
    }
    SDL_DestroySurface(surf);
}

void draw_texture(SDL_Renderer* renderer, SDL_Texture* texture, float x, float y, float w, float h) {
    if (!texture) return;
    SDL_FRect dst = { x, y, w, h };
    SDL_RenderTexture(renderer, texture, NULL, &dst);
}
