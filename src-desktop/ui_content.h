#ifndef UI_CONTENT_H
#define UI_CONTENT_H

#include <SDL3/SDL.h>
#include "app_state.h"
#include "assets.h"

#ifdef __cplusplus
extern "C" {
#endif

void ui_content_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets);
bool ui_content_handle_click(AppState* state, float x, float y);

#ifdef __cplusplus
}
#endif

#endif // UI_CONTENT_H
