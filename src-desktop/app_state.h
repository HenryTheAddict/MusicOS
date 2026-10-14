#ifndef APP_STATE_H
#define APP_STATE_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    MODE_LIBRARY = 0,
    MODE_DEVICES = 1
} AppMode;

typedef enum {
    SECTION_ALBUMS = 0,
    SECTION_ARTISTS = 1,
    SECTION_GENRES = 2,
    SECTION_SONGS = 3
} LibrarySection;

typedef struct {
    const char* title;
    const char* artist;
    float progress; // 0.0 to 1.0
    bool is_playing;
    bool repeat_on;
    bool shuffle_on;
    bool lossless;
} NowPlayingInfo;

typedef struct {
    const char* title;
    const char* artist;
    const char* cover_path;
} AlbumCard;

typedef struct {
    const char* title;
    const char* subtitle;
    const char* cover_path;
    bool is_sparklist;
} PlaylistItem;

typedef struct {
    AppMode mode;
    LibrarySection section;
    bool sparklist_modal_open;
    int selected_album;
    NowPlayingInfo now_playing;
    AlbumCard albums[6];
    PlaylistItem playlists[4];
} AppState;

void app_state_init(AppState* state);
void app_state_toggle_mode(AppState* state);
void app_state_select_section(AppState* state, LibrarySection section);
void app_state_toggle_playback(AppState* state);
void app_state_toggle_repeat(AppState* state);
void app_state_toggle_shuffle(AppState* state);

#ifdef __cplusplus
}
#endif

#endif // APP_STATE_H
