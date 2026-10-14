#ifndef UI_TOOLBAR_H
#define UI_TOOLBAR_H

#include <SDL3/SDL.h>
#include "app_state.h"
#include "assets.h"

#ifdef __cplusplus
extern "C" {
#endif

void ui_toolbar_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets);
bool ui_toolbar_handle_click(AppState* state, float x, float y, bool* should_quit, bool* should_minimize);

#ifdef __cplusplus
}
#endif

#endif // UI_TOOLBAR_H
