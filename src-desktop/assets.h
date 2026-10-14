#ifndef ASSETS_H
#define ASSETS_H

#include <SDL3/SDL.h>
#include <SDL3_ttf/SDL_ttf.h>
#include <stdbool.h>

typedef struct {
    TTF_Font* font_8;   // For 8px subtitles: "Solitude maybe? - HelloWorld", "3h21m - 400 songs", "artist"
    TTF_Font* font_11;  // For 12px text: "Treelist", "Favorites", "{user}s stuff:"
    TTF_Font* font_13;  // For 14px text: "Albums", "Artists", "Genres", "Songs", "New:", "Recently Added", "Album Name"
    TTF_Font* font_20;  // For 24px headers: "Library", "Search!"
} FontAssets;

typedef struct {
    // Top Bar & Icons
    SDL_Texture* ic_3dots;
    SDL_Texture* ic_tiny_x;
    SDL_Texture* ic_tiny_min;
    SDL_Texture* ic_tiny_max;
    SDL_Texture* ic_queue;
    SDL_Texture* ic_lossless;
    SDL_Texture* ic_repeat_on;
    SDL_Texture* ic_repeat_off;
    SDL_Texture* ic_shuffle_on;
    SDL_Texture* ic_shuffle_off;
    SDL_Texture* ic_prev;
    SDL_Texture* ic_play;
    SDL_Texture* ic_pause;
    SDL_Texture* ic_next;

    // Sidebar Icons
    SDL_Texture* ic_album;
    SDL_Texture* ic_artist;
    SDL_Texture* ic_genre;
    SDL_Texture* ic_song;
    SDL_Texture* ic_playlist;
    SDL_Texture* ic_sparklist;
    SDL_Texture* ic_shuffle_play;
    SDL_Texture* ic_device;
    SDL_Texture* ic_3dots_incircle;
    SDL_Texture* ic_netsearch;
    SDL_Texture* ic_file;

    // Artwork Covers
    SDL_Texture* cv_sky;
    SDL_Texture* cv_tritower;
    SDL_Texture* cv_overcast;
    SDL_Texture* cv_sparklist_1;
} TextureAssets;

typedef struct {
    FontAssets fonts;
    TextureAssets textures;
} AppAssets;

bool assets_init(AppAssets* assets, SDL_Renderer* renderer);
void assets_destroy(AppAssets* assets);

#endif // ASSETS_H
