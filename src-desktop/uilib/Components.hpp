#pragma once
#include "Types.hpp"
#include "Canvas.hpp"
#include "Animation.hpp"
#include <SDL3/SDL.h>
#include <string>
#include <vector>
#include <memory>
#include <functional>

namespace MusicUI {
struct DemoLibrary;
struct CatalogBatch;

// Base Widget Class
class Widget {
public:
    Rect bounds;
    bool visible = true;

    Widget(const Rect& b = Rect()) : bounds(b) {}
    virtual ~Widget() = default;

    virtual void render(Canvas& canvas) {
        if (visible) render_self(canvas);
    }
    virtual void render_self(Canvas& /*canvas*/) {}
    virtual bool handle_mouse_down(float /*x*/, float /*y*/, int /*button*/) { return false; }
    virtual bool handle_mouse_up(float /*x*/, float /*y*/, int /*button*/) { return false; }
    virtual bool handle_mouse_move(float /*x*/, float /*y*/) { return false; }
};

// Generic Button
class Button : public Widget {
public:
    std::string text;
    std::string icon_path;
    Color bg_color = Color::PrimaryBlue;
    Color hover_bg = Color::DeepBlue;
    Color text_color = Color::White;
    Color border_color = Color::Transparent;
    float border_width = 0.0f;
    float corner_radius = 8.0f;
    float font_size = 14.0f;
    FontWeight font_weight = FontWeight::Medium;
    bool hovered = false;
    bool enabled = true;
    bool has_gradient = false;
    LinearGradient gradient;

    std::function<void()> on_click;

    Button(const Rect& b = Rect(), const std::string& txt = "");
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    bool handle_mouse_move(float x, float y) override;
};

// 1. macOS Traffic Light Controls (Red, Yellow, Green)
class TrafficLights : public Widget {
public:
    std::function<void()> on_close;
    std::function<void()> on_minimize;
    std::function<void()> on_maximize;
    bool hovered = false;

    TrafficLights(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    bool handle_mouse_move(float x, float y) override;
};

// 2. Scope Selector ("Library" | "Devices")
class ScopeSelector : public Widget {
public:
    enum class Mode { Library, Devices, Settings };
    Mode mode = Mode::Library;
    bool menu_open = false, hovered = false;
    int menu_hover = -1, menu_focus = 0;
    std::function<void(Mode)> on_mode_change;
    SpringAnim anim_press{1.0f, 650.0f, 51.0f};
    TimedAnim scope_morph{0.0f, 0.16f};
    TimedAnim menu_reveal{0.0f, 0.14f};
    ScopeSelector(const Rect& b);
    Rect menu_bounds() const { return {bounds.x,bounds.bottom()+8,bounds.w,140}; }
    void render_self(Canvas& canvas) override;
    void render_menu(Canvas& canvas);
    void close_menu() { menu_open=false; menu_reveal.set_target(0); }
    bool handle_mouse_down(float x, float y, int button) override;
    bool handle_mouse_move(float x, float y) override;
    bool handle_key_down(SDL_Keycode key);
    void update_animations(float dt) {
        anim_press.update(dt); scope_morph.update(dt); menu_reveal.update(dt);
    }
    bool is_animating() const { return anim_press.is_animating() || scope_morph.is_animating() || menu_reveal.is_animating(); }

};

// Playback State
struct PlaybackState {
    std::string title = "Plastic Beach";
    std::string artist = "Gorillaz";
    std::string cover_path = "assets/covers/playlists/pl_0.png";
    float progress = 0.35f;
    bool is_playing = false; // Playback starts from an explicit action
    bool repeat_on = false;
    bool shuffle_on = false;
};

// 3. Now Playing Pill with Scrubber & Playback Controls
class NowPlayingPill : public Widget {
public:
    PlaybackState state;
    std::function<void()> on_prev;
    std::function<void()> on_play_pause;
    std::function<void()> on_next;
    std::function<void()> on_repeat_toggle;
    std::function<void()> on_shuffle_toggle;
    std::function<void(float)> on_seek;

    SpringAnim play_pop{1.0f, 650.0f, 51.0f};
    SpringAnim next_pop{1.0f, 650.0f, 51.0f};
    SpringAnim prev_pop{1.0f, 650.0f, 51.0f};
    TimedAnim play_morph{1.0f, 0.16f};    // 0.4s Play ⇄ Pause vector morph
    TimedAnim note_morph{1.0f, 0.16f};    // 0.4s Music Note ⇄ Waveform morph
    TimedAnim repeat_morph{0.0f, 0.16f};  // 0.4s Repeat OFF ⇄ ON morph
    TimedAnim shuffle_morph{0.0f, 0.16f}; // 0.4s Shuffle OFF ⇄ ON morph
    float wave_phase = 0.0f;

    bool is_scrubbing = false;

    NowPlayingPill(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    bool handle_mouse_move(float x, float y) override;
    bool handle_mouse_up(float x, float y, int button) override;
    void update_animations(float dt) {
        play_pop.update(dt);
        next_pop.update(dt);
        prev_pop.update(dt);
        play_morph.update(dt);
        note_morph.update(dt);
        repeat_morph.update(dt);
        shuffle_morph.update(dt);
        if (state.is_playing) {
            wave_phase += dt * 3.5f;
        }
    }
    bool is_animating() const {
        return play_pop.is_animating() || next_pop.is_animating() || prev_pop.is_animating() ||
               play_morph.is_animating() || note_morph.is_animating() ||
               repeat_morph.is_animating() || shuffle_morph.is_animating() || state.is_playing;
    }
};

// 4. View Mode Selector (Icons | List)
class ViewModeSelector : public Widget {
public:
    enum class View { Icons, List };
    View view = View::Icons;
    std::function<void(View)> on_view_change;

    ViewModeSelector(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
};

// 5. Puffy Toggle (Auto-Manage vs Manual Manage Segmented Pill)
class PuffyToggle : public Widget {
public:
    enum class Selection { AutoManage, ManualManage };
    Selection selection = Selection::ManualManage;
    std::function<void(Selection)> on_selection_change;
    SpringAnim anim_x{208.0f, 580.0f, 38.0f};

    PuffyToggle(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    void update_animations(float dt) { anim_x.update(dt); }
    bool is_animating() const { return anim_x.is_animating(); }
};

// 6. Category Tab Bar (Artists | Albums | Playlists | Songs)
class CategoryTabBar : public Widget {
public:
    int selected_index = 0;
    bool show_search_bar = true;
    std::function<void(int)> on_tab_change;
    SpringAnim anim_x{0.0f, 580.0f, 38.0f};

    CategoryTabBar(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    void update_animations(float dt) { anim_x.update(dt); }
    bool is_animating() const { return anim_x.is_animating(); }
};

// 7. Qualitizer Card (Lossless/OG, Opus 128, Opus 256, Opus 320)
class QualitizerCard : public Widget {
public:
    int selected_rate = 0; // 0=Lossless/OG, 1=Opus 128, 2=Opus 256, 3=Opus 320
    std::function<void(int)> on_rate_change;
    SpringAnim anim_x{0.0f, 580.0f, 38.0f};

    QualitizerCard(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    void update_animations(float dt) { anim_x.update(dt); }
    bool is_animating() const { return anim_x.is_animating(); }
};

// 8. Manual Sync Management Card (Tabs, Search, Filter, 8 Artist Rows)
class ManualSyncCard : public Widget {
public:
    std::shared_ptr<CategoryTabBar> tab_bar;
    bool checked[8] = { true, true, false, true, true, true, true, false };
    std::function<void(int count)> on_sync_change;

    ManualSyncCard(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
};

// 9. Sparklist Modal Dialog
class SparklistModal : public Widget {
public:
    struct SparkSong {
        std::string title;
        std::string artist;
        std::string cover_path;
        int vibe_idx;
    };

    std::vector<SparkSong> candidates;
    std::function<void()> on_close;
    std::function<void()> on_choose_for_me;
    std::function<void(const SparkSong& song)> on_song_chosen;
    TimedAnim sparkle_morph{0.0f, 0.16f};

    SparklistModal(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    void update_animations(float dt) { sparkle_morph.update(dt); }
    bool is_animating() const { return sparkle_morph.is_animating(); }
};

// 10. Album Grid
class AlbumGrid : public Widget {
public:
    static constexpr int MAX_ALBUMS = 64;
    int selected_index = -1;
    int hovered_index = -1;
    bool show_search_bar = true;
    std::function<void(int)> on_album_select;
    std::function<void(const std::string&)> on_artist_select;
    std::function<void(const CatalogBatch&)> on_catalog_results;
    std::function<void(const std::string&,int,const std::string&)> on_search_catalog;
    DemoLibrary* library=nullptr;
    int category=0;
    std::string genre_filter,active_request,playing_track;
    bool pending_search=false;
    float search_debounce=0;
    Uint64 last_request=0;
    void rebuild();
    void apply_catalog(const CatalogBatch& batch);
    void open_genre(std::string genre);
    void tick_search(float dt);
    struct Hit {Rect rect;std::function<void()> action;};
    std::vector<Hit> content_hits;
    struct AlbumCard {
        std::string title;
        std::string artist;
        std::string cover_path;
        std::string id;
        enum class Kind { Album, Artist, Genre, Song };
        Kind kind=Kind::Album;
    };
    std::vector<AlbumCard> all_cards;
    std::vector<AlbumCard> cards;
    SpringAnim card_scale[MAX_ALBUMS];

    // Scrolling support
    float scroll_y = 0.0f;
    float target_scroll_y = 0.0f;
    float max_scroll_y = 0.0f;
    bool is_dragging_scroll = false;
    float drag_start_y = 0.0f;
    float drag_start_scroll = 0.0f;

    // Interactive Search with C libcurl scraping
    std::string search_query = "";
    bool search_active = false;
    bool is_searching = false;
    std::string search_status = "";
    float cursor_blink_time = 0.0f;

    void set_category(int cat_idx);
    void handle_wheel(float dy);
    void set_search_query(const std::string& query);
    void trigger_search(const std::string& query);
    void ingest_pending_results();
    bool handle_key_down(SDL_Keycode key);
    bool handle_text_input(const char* text);

    AlbumGrid(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    bool handle_mouse_move(float x, float y) override;
    bool handle_mouse_up(float x, float y, int button) override;
    void update_animations(float dt) {
        ingest_pending_results();tick_search(dt);
        for (int i = 0; i < MAX_ALBUMS; ++i) card_scale[i].update(dt);
        float diff = target_scroll_y - scroll_y;
        if (std::abs(diff) > 0.2f) {
            scroll_y += diff * (1.f-std::exp(-18.f*dt));
        } else {
            scroll_y = target_scroll_y;
        }
        cursor_blink_time += dt;
    }
    bool is_animating() const {
        if(pending_search || search_active)return true;
        if (std::abs(target_scroll_y - scroll_y) > 0.2f) return true;
        for (int i = 0; i < MAX_ALBUMS; ++i) {
            if (card_scale[i].is_animating()) return true;
        }
        return false;
    }
};

// 11. Left Sidebar (Supports Library Mode and Devices Mode)
class Sidebar : public Widget {
public:
    std::string display_name = "Henry";
    std::string back_label;
    std::function<void()> on_back;
    enum class Mode { Library, Devices, Settings };
    Mode mode = Mode::Library;
    int nav_selected = 0; // 0=Albums, 1=Artists, 2=Genres, 3=Songs
    int selected_playlist = 2; // Favorites

    std::function<void()> on_create_playlist_click;
    std::function<void()> on_create_sparklist_click;
    std::function<void(int)> on_nav_change;
    std::function<void(int)> on_playlist_select;

    TimedAnim storage_anim{50.8f, 0.16f}; // 0.4s smooth storage bar morph
    TimedAnim battery_morph{0.0f, 0.16f}; // 0.4s Battery ⇄ Lightning morph
    std::string storage_text = "Music 01 - 256GB - 200GB Free";
    int current_rate = 0;
    int current_sync_count = 6;

    void set_storage_rate(int rate);
    void set_sync_count(int count);
    void recalculate_storage();

    Sidebar(const Rect& b);
    void render_self(Canvas& canvas) override;
    bool handle_mouse_down(float x, float y, int button) override;
    void update_animations(float dt) {
        storage_anim.update(dt);
        battery_morph.update(dt);
    }
    bool is_animating() const { return storage_anim.is_animating() || battery_morph.is_animating(); }
};

} // namespace MusicUI
