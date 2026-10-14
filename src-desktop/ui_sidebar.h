#ifndef UI_SIDEBAR_H
#define UI_SIDEBAR_H

#include <SDL3/SDL.h>
#include "app_state.h"
#include "assets.h"

#ifdef __cplusplus
extern "C" {
#endif

void ui_sidebar_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets);
bool ui_sidebar_handle_click(AppState* state, float x, float y);

#ifdef __cplusplus
}
#endif

#endif // UI_SIDEBAR_H
