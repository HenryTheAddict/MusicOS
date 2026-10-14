#include "assets.h"
#include <SDL3_image/SDL_image.h>
#include <stdio.h>

static SDL_Texture* assets_load_texture(SDL_Renderer* renderer, const char* path) {
    SDL_Texture* tex = IMG_LoadTexture(renderer, path);
    if (!tex) {
        // Warning: fallback gracefully
        // printf("Warning: Failed to load texture: %s\n", path);
    }
    return tex;
}

bool assets_init(AppAssets* assets, SDL_Renderer* renderer) {
    // 1. Load fonts
    assets->fonts.font_8 = TTF_OpenFont("assets/fonts/RasterGothic12Cond.ttf", 9);
    if (!assets->fonts.font_8) {
        assets->fonts.font_8 = TTF_OpenFont("assets/fonts/Figtree-Variable.ttf", 10);
    }

    assets->fonts.font_11 = TTF_OpenFont("assets/fonts/RasterGothic14Cond.ttf", 11);
    if (!assets->fonts.font_11) {
        assets->fonts.font_11 = TTF_OpenFont("assets/fonts/Figtree-Variable.ttf", 12);
    }

    assets->fonts.font_13 = TTF_OpenFont("assets/fonts/RasterGothic18Cond.ttf", 13);
    if (!assets->fonts.font_13) {
        assets->fonts.font_13 = TTF_OpenFont("assets/fonts/Figtree-Variable.ttf", 14);
    }

    assets->fonts.font_20 = TTF_OpenFont("assets/fonts/RasterGothic24CondBold.ttf", 20);
    if (!assets->fonts.font_20) {
        assets->fonts.font_20 = TTF_OpenFont("assets/fonts/Figtree-Variable.ttf", 20);
    }

    // 2. Load SVG icons from assets/icons/sketch/
    assets->textures.ic_3dots = assets_load_texture(renderer, "assets/icons/sketch/3dots_(nocircle).svg");
    assets->textures.ic_tiny_x = assets_load_texture(renderer, "assets/icons/sketch/tinyX.svg");
    assets->textures.ic_tiny_min = assets_load_texture(renderer, "assets/icons/sketch/tinyMinimize.svg");
    assets->textures.ic_tiny_max = assets_load_texture(renderer, "assets/icons/sketch/tinyMaximize.svg");
    assets->textures.ic_queue = assets_load_texture(renderer, "assets/icons/sketch/queue.svg");
    assets->textures.ic_lossless = assets_load_texture(renderer, "assets/icons/sketch/Lossless.svg");
    assets->textures.ic_repeat_on = assets_load_texture(renderer, "assets/icons/sketch/RepeatON.svg");
    assets->textures.ic_repeat_off = assets_load_texture(renderer, "assets/icons/sketch/RepeatOFF.svg");
    assets->textures.ic_shuffle_on = assets_load_texture(renderer, "assets/icons/sketch/ShuffleON.svg");
    assets->textures.ic_shuffle_off = assets_load_texture(renderer, "assets/icons/sketch/ShuffleOFF.svg");
    assets->textures.ic_prev = assets_load_texture(renderer, "assets/icons/sketch/prev.svg");
    assets->textures.ic_play = assets_load_texture(renderer, "assets/icons/sketch/play.svg");
    assets->textures.ic_pause = assets_load_texture(renderer, "assets/icons/sketch/pause.svg");
    assets->textures.ic_next = assets_load_texture(renderer, "assets/icons/sketch/next.svg");

    assets->textures.ic_album = assets_load_texture(renderer, "assets/icons/sketch/Album.svg");
    assets->textures.ic_artist = assets_load_texture(renderer, "assets/icons/sketch/Artist.svg");
    assets->textures.ic_genre = assets_load_texture(renderer, "assets/icons/sketch/Genre.svg");
    assets->textures.ic_song = assets_load_texture(renderer, "assets/icons/sketch/Song.svg");
    assets->textures.ic_playlist = assets_load_texture(renderer, "assets/icons/sketch/Playlist.svg");
    assets->textures.ic_sparklist = assets_load_texture(renderer, "assets/icons/sketch/Sparklist.svg");
    assets->textures.ic_shuffle_play = assets_load_texture(renderer, "assets/icons/sketch/ShufflePlay.svg");
    assets->textures.ic_device = assets_load_texture(renderer, "assets/icons/sketch/device.svg");
    assets->textures.ic_3dots_incircle = assets_load_texture(renderer, "assets/icons/sketch/3dots(incircle).svg");
    assets->textures.ic_netsearch = assets_load_texture(renderer, "assets/icons/sketch/NetSearch.svg");
    assets->textures.ic_file = assets_load_texture(renderer, "assets/icons/sketch/File.svg");

    // 3. Load cover art from assets/covers/
    assets->textures.cv_sky = assets_load_texture(renderer, "assets/covers/sky.png");
    assets->textures.cv_tritower = assets_load_texture(renderer, "assets/covers/tritower.png");
    assets->textures.cv_overcast = assets_load_texture(renderer, "assets/covers/Overcast.png");
    assets->textures.cv_sparklist_1 = assets_load_texture(renderer, "assets/covers/sparklist_1.png");

    return true;
}

void assets_destroy(AppAssets* assets) {
    if (assets->fonts.font_8) TTF_CloseFont(assets->fonts.font_8);
    if (assets->fonts.font_11) TTF_CloseFont(assets->fonts.font_11);
    if (assets->fonts.font_13) TTF_CloseFont(assets->fonts.font_13);
    if (assets->fonts.font_20) TTF_CloseFont(assets->fonts.font_20);

    if (assets->textures.ic_3dots) SDL_DestroyTexture(assets->textures.ic_3dots);
    if (assets->textures.ic_tiny_x) SDL_DestroyTexture(assets->textures.ic_tiny_x);
    if (assets->textures.ic_tiny_min) SDL_DestroyTexture(assets->textures.ic_tiny_min);
    if (assets->textures.ic_tiny_max) SDL_DestroyTexture(assets->textures.ic_tiny_max);
    if (assets->textures.ic_queue) SDL_DestroyTexture(assets->textures.ic_queue);
    if (assets->textures.ic_lossless) SDL_DestroyTexture(assets->textures.ic_lossless);
    if (assets->textures.ic_repeat_on) SDL_DestroyTexture(assets->textures.ic_repeat_on);
    if (assets->textures.ic_repeat_off) SDL_DestroyTexture(assets->textures.ic_repeat_off);
    if (assets->textures.ic_shuffle_on) SDL_DestroyTexture(assets->textures.ic_shuffle_on);
    if (assets->textures.ic_shuffle_off) SDL_DestroyTexture(assets->textures.ic_shuffle_off);
    if (assets->textures.ic_prev) SDL_DestroyTexture(assets->textures.ic_prev);
    if (assets->textures.ic_play) SDL_DestroyTexture(assets->textures.ic_play);
    if (assets->textures.ic_pause) SDL_DestroyTexture(assets->textures.ic_pause);
    if (assets->textures.ic_next) SDL_DestroyTexture(assets->textures.ic_next);

    if (assets->textures.ic_album) SDL_DestroyTexture(assets->textures.ic_album);
    if (assets->textures.ic_artist) SDL_DestroyTexture(assets->textures.ic_artist);
    if (assets->textures.ic_genre) SDL_DestroyTexture(assets->textures.ic_genre);
    if (assets->textures.ic_song) SDL_DestroyTexture(assets->textures.ic_song);
    if (assets->textures.ic_playlist) SDL_DestroyTexture(assets->textures.ic_playlist);
    if (assets->textures.ic_sparklist) SDL_DestroyTexture(assets->textures.ic_sparklist);
    if (assets->textures.ic_shuffle_play) SDL_DestroyTexture(assets->textures.ic_shuffle_play);
    if (assets->textures.ic_device) SDL_DestroyTexture(assets->textures.ic_device);
    if (assets->textures.ic_3dots_incircle) SDL_DestroyTexture(assets->textures.ic_3dots_incircle);
    if (assets->textures.ic_netsearch) SDL_DestroyTexture(assets->textures.ic_netsearch);
    if (assets->textures.ic_file) SDL_DestroyTexture(assets->textures.ic_file);

    if (assets->textures.cv_sky) SDL_DestroyTexture(assets->textures.cv_sky);
    if (assets->textures.cv_tritower) SDL_DestroyTexture(assets->textures.cv_tritower);
    if (assets->textures.cv_overcast) SDL_DestroyTexture(assets->textures.cv_overcast);
    if (assets->textures.cv_sparklist_1) SDL_DestroyTexture(assets->textures.cv_sparklist_1);
}
