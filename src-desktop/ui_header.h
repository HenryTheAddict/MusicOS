#ifndef UI_HEADER_H
#define UI_HEADER_H

#include <SDL3/SDL.h>
#include "app_state.h"
#include "assets.h"

#ifdef __cplusplus
extern "C" {
#endif

void ui_header_render(SDL_Renderer* renderer, const AppState* state, const AppAssets* assets);

#ifdef __cplusplus
}
#endif

#endif // UI_HEADER_H
