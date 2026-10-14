#ifndef DRAW_UTILS_H
#define DRAW_UTILS_H

#include <SDL3/SDL.h>
#include <SDL3_ttf/SDL_ttf.h>
#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Basic shapes
void draw_rect_filled(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t hex);
void draw_rect_filled_rgba(SDL_Renderer* renderer, float x, float y, float w, float h, Uint8 r, Uint8 g, Uint8 b, Uint8 a);
void draw_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t top_hex, uint32_t bot_hex);
void draw_rounded_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t top_hex, uint32_t bot_hex);
void draw_rounded_top_rect_vertical_gradient(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t top_hex, uint32_t bot_hex);
void draw_rect_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float thickness, uint32_t hex);

void draw_circle_filled(SDL_Renderer* renderer, float cx, float cy, float radius, uint32_t hex);
void draw_circle_outline(SDL_Renderer* renderer, float cx, float cy, float radius, float thickness, uint32_t hex);

void draw_rounded_rect_filled(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, uint32_t hex);
void draw_rounded_rect_filled_rgba(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, Uint8 r, Uint8 g, Uint8 b, Uint8 a);
void draw_rounded_rect_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float radius, float thickness, uint32_t hex);

// Convenience
void draw_pill_filled(SDL_Renderer* renderer, float x, float y, float w, float h, uint32_t hex);
void draw_pill_outline(SDL_Renderer* renderer, float x, float y, float w, float h, float thickness, uint32_t hex);

// Text rendering
void draw_text(SDL_Renderer* renderer, TTF_Font* font, const char* text, float x, float y, uint32_t hex);
void draw_text_centered(SDL_Renderer* renderer, TTF_Font* font, const char* text, float cx, float cy, uint32_t hex);
void draw_text_in_box(SDL_Renderer* renderer, TTF_Font* font, const char* text, float x, float y, float w, float h, bool center_x, bool center_y, uint32_t hex);

void draw_texture(SDL_Renderer* renderer, SDL_Texture* texture, float x, float y, float w, float h);

#ifdef __cplusplus
}
#endif

#endif // DRAW_UTILS_H
