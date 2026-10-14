#include "app_state.h"
#include <string.h>

void app_state_init(AppState* state) {
    if (!state) return;
    memset(state, 0, sizeof(AppState));
    state->mode = MODE_LIBRARY;
    state->section = SECTION_ALBUMS;
    state->sparklist_modal_open = false;
    state->selected_album = 0;

    state->now_playing.title = "Imminent Reccolection";
    state->now_playing.artist = "Solitude maybe? - HelloWorld";
    state->now_playing.progress = 42.0f / 145.0f; // ~0.29
    state->now_playing.is_playing = true;
    state->now_playing.repeat_on = false;
    state->now_playing.shuffle_on = true;
    state->now_playing.lossless = true;

    // 6 Album cards matching Sketch
    for (int i = 0; i < 6; i++) {
        state->albums[i].title = "Album Name";
        state->albums[i].artist = "artist";
        state->albums[i].cover_path = NULL; // fallback to solid black square
    }

    // 4 Playlists matching Sketch
    state->playlists[0].title = "Treelist";
    state->playlists[0].subtitle = "3h21m - 400 songs";
    state->playlists[0].cover_path = "assets/covers/sky.png";
    state->playlists[0].is_sparklist = false;

    state->playlists[1].title = "My Playlist40";
    state->playlists[1].subtitle = "40m - 10 songs";
    state->playlists[1].cover_path = "assets/covers/tritower.png";
    state->playlists[1].is_sparklist = false;

    state->playlists[2].title = "Favorites";
    state->playlists[2].subtitle = "Sparklist …";
    state->playlists[2].cover_path = "assets/covers/sparklist_1.png";
    state->playlists[2].is_sparklist = true;

    state->playlists[3].title = "My Playlist8";
    state->playlists[3].subtitle = "40m - 10 songs";
    state->playlists[3].cover_path = "assets/covers/Overcast.png";
    state->playlists[3].is_sparklist = false;
}

void app_state_toggle_mode(AppState* state) {
    if (!state) return;
    state->mode = (state->mode == MODE_LIBRARY) ? MODE_DEVICES : MODE_LIBRARY;
}

void app_state_select_section(AppState* state, LibrarySection section) {
    if (!state) return;
    state->section = section;
}

void app_state_toggle_playback(AppState* state) {
    if (!state) return;
    state->now_playing.is_playing = !state->now_playing.is_playing;
}

void app_state_toggle_repeat(AppState* state) {
    if (!state) return;
    state->now_playing.repeat_on = !state->now_playing.repeat_on;
}

void app_state_toggle_shuffle(AppState* state) {
    if (!state) return;
    state->now_playing.shuffle_on = !state->now_playing.shuffle_on;
}
