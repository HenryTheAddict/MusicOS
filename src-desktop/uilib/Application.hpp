#pragma once
#include "Types.hpp"
#include "Canvas.hpp"
#include "Components.hpp"
#include "LibraryExperience.hpp"
#include <SDL3/SDL.h>
#include <memory>
#include <string>

namespace MusicUI {

class Application {
    friend struct DesktopChecks;
public:
    Application();
    ~Application();

    bool init(int argc, char* argv[]);
    void run();

    void set_mode(Sidebar::Mode mode);
    void show_sparklist_modal(bool show);
    void save_screenshot(const std::string& path);
    void resize_buffers(int new_render_w, int new_render_h);
    void set_screen_pixel_size(int target_px_w, int target_px_h);
    void set_window_size_points(int points_w, int points_h);
    void toggle_maximize();
    void update_layout();

private:
    void render_frame();
    void handle_event(const SDL_Event& event);
    void update_animations(float dt);
    SDL_HitTestResult hit_test(int x, int y);
    static SDL_HitTestResult SDLCALL hit_test_cb(SDL_Window* win, const SDL_Point* area, void* data);
    bool is_interactive_point(float x, float y) const;
    int resize_edges(float x,float y) const;
    int m_resize_edges=0,m_resize_x=0,m_resize_y=0,m_resize_w=0,m_resize_h=0;
    float m_resize_mouse_x=0,m_resize_mouse_y=0;
    SDL_Cursor* m_resize_cursors[4]={};

    SpringAnim m_modal_scale{0.88f, 580.0f, 36.0f};
    SpringAnim m_modal_alpha{0.0f, 550.0f, 40.0f};
    SpringAnim m_view_transition{0.0f, 520.0f, 36.0f};

    void update_lib_surface();
    void update_dev_surface();
    void update_modal_surface();
    void update_blur_surface();

    SDL_Window* m_window = nullptr;
    SDL_Renderer* m_renderer = nullptr;
    SDL_Texture* m_texture = nullptr;
    SDL_Cursor* m_cursor_default = nullptr;
    SDL_Cursor* m_cursor_pointer = nullptr;

    cairo_surface_t* m_cairo_surface = nullptr;
    cairo_surface_t* m_blurred_bg_surface = nullptr;
    cairo_surface_t* m_lib_surface = nullptr;
    cairo_surface_t* m_dev_surface = nullptr;
    cairo_surface_t* m_modal_surface = nullptr;
    bool m_modal_bg_cached = false;
    bool m_lib_dirty = true;
    bool m_dev_dirty = true;
    bool m_modal_dirty = true;
    std::unique_ptr<Canvas> m_canvas;

    bool m_running = true;
    bool m_show_modal = false;
    bool m_needs_redraw = true;
    bool has_active_animations() const;
    std::string m_dump_png;
    Sidebar::Mode m_mode = Sidebar::Mode::Library;

    LibraryExperience m_experience;
    void preferences_changed();
    void sync_demo_player();
    void render_library();
    void update_back_label();
    std::string m_initial_view;
    bool m_snapshot_colors = false;
    int m_snapshot_percent = 0;
    // UI Widgets
    std::shared_ptr<TrafficLights> m_traffic_lights;
    std::shared_ptr<ScopeSelector> m_scope_selector;
    std::shared_ptr<NowPlayingPill> m_now_playing;
    std::shared_ptr<Sidebar> m_sidebar;
    std::shared_ptr<AlbumGrid> m_album_grid;

    // Devices Mode Content Widgets
    std::shared_ptr<PuffyToggle> m_puffy_toggle;
    std::shared_ptr<ManualSyncCard> m_manual_sync_card;
    std::shared_ptr<QualitizerCard> m_qualitizer_card;

    // Modal Widget
    std::shared_ptr<SparklistModal> m_modal;

    int m_logical_w = 720;
    int m_logical_h = 480;
    int m_render_w = 1440;
    int m_render_h = 960;
    float m_scale = 2.0f;
};

} // namespace MusicUI
