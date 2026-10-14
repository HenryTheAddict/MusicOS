#pragma once
#include "Components.hpp"
#include "LibraryModel.hpp"
#include <unordered_map>

namespace MusicUI {
struct ArtworkPalette { Color primary = Color::from_hex(0x00AEFF), secondary = Color::from_hex(0x0064DF); bool valid = false; };
class LibraryExperience {
public:
    DemoLibrary library;
    Preferences preferences;
    LibraryNavigation navigation;
    DemoSequence sequence;
    std::string preferences_path, save_error, data_status;
    std::function<void(const std::string&,bool)> on_catalog_lookup;
    std::function<void()> on_preferences_changed;
    std::function<void()> on_playback_changed;
    bool settings = false;
    bool editing_name = false;
    bool select_name = false;
    bool menu_open = false;
    std::string name_draft;
    float settings_scroll = 0;
    float theme_mix = 1;
    LibraryExperience();
    ~LibraryExperience();
    void render(Canvas& canvas, Rect bounds);
    bool click(float x, float y);
    void pointer_move(float x, float y);
    void pointer_up() { dragging_scroll=false; }
    bool key(SDL_Keycode key, SDL_Keymod modifiers);
    void text(const std::string& input);
    void scroll(float dy);
    void open(LibraryPage page, const std::string& id);
    bool back();
    void start(bool shuffle = false, const std::string& track = {});
    void advance(int delta);
    void save_preferences();
    bool commit_name();
    void update(float dt, const std::string& playing_cover, bool library_active = true);
    Color remap(const Color& color) const;
    void render_background(Canvas& canvas, Rect bounds);
    bool animating() const { return theme_mix < 1 || page_reveal.is_animating() || toggle_position.is_animating() || scroll_position.is_animating(); }
    void reveal() { page_reveal.snap(0);page_reveal.set_target(1);scroll_position.snap(settings?settings_scroll:navigation.current.scroll); }
    float content_height = 0;
private:
    struct Hit { Rect rect; std::function<void()> action; };
    std::vector<Hit> hits, menu_hits;
    Rect rendered_bounds, menu_rect, scroll_rail;
    float scroll_thumb=0;
    bool dragging_scroll=false;
    std::string menu_track, artwork_key, previous_key, playing_artwork;
    Point pointer{-100,-100};
    TimedAnim page_reveal{1,.16f}, toggle_position{0,.14f};
    SpringAnim scroll_position{0,650,51};
    std::string last_route;
    std::unordered_map<std::string, ArtworkPalette> palettes;
    std::unordered_map<std::string, cairo_surface_t*> backgrounds;
    ArtworkPalette current_palette, previous_palette;
    ArtworkPalette palette(const std::string& path);
    cairo_surface_t* background(const std::string& path);
    void render_settings(Canvas& canvas, Rect bounds);
    void render_album(Canvas& canvas, Rect bounds);
    void render_artist(Canvas& canvas, Rect bounds);
    void render_menu(Canvas& canvas);
    void add_hit(Rect r, std::function<void()> action);
    void icon_button(Canvas& canvas, Rect r, const std::string& icon, std::function<void()> action, bool pill = false);
    void overflow(Canvas& canvas, Rect r, const std::string& track = {});
};
}
