#include "Application.hpp"
#include "AudioEngine.hpp"
#include "MusicSearchService.hpp"
#include <iostream>
#include <cstring>

namespace MusicUI {

Application::Application() {}

Application::~Application() {
    MusicSearchService::instance().shutdown();
    AudioEngine::instance().shutdown();

    m_modal.reset();
    m_qualitizer_card.reset();
    m_manual_sync_card.reset();
    m_puffy_toggle.reset();
    m_album_grid.reset();
    m_sidebar.reset();
    m_now_playing.reset();
    m_scope_selector.reset();
    m_traffic_lights.reset();

    m_canvas.reset();

    if (m_modal_surface) { cairo_surface_destroy(m_modal_surface); m_modal_surface = nullptr; }
    if (m_lib_surface) { cairo_surface_destroy(m_lib_surface); m_lib_surface = nullptr; }
    if (m_dev_surface) { cairo_surface_destroy(m_dev_surface); m_dev_surface = nullptr; }
    if (m_blurred_bg_surface) { cairo_surface_destroy(m_blurred_bg_surface); m_blurred_bg_surface = nullptr; }
    if (m_cairo_surface) { cairo_surface_destroy(m_cairo_surface); m_cairo_surface = nullptr; }

    if (m_texture) { SDL_DestroyTexture(m_texture); m_texture = nullptr; }
    if (m_renderer) { SDL_DestroyRenderer(m_renderer); m_renderer = nullptr; }
    if (m_cursor_pointer) { SDL_DestroyCursor(m_cursor_pointer); m_cursor_pointer = nullptr; }
    if (m_cursor_default) { SDL_DestroyCursor(m_cursor_default); m_cursor_default = nullptr; }
    for(auto cursor:m_resize_cursors)if(cursor)SDL_DestroyCursor(cursor);
    if (m_window) { SDL_DestroyWindow(m_window); m_window = nullptr; }

    SDL_Quit();
}

bool Application::init(int argc, char* argv[]) {
    // Parse CLI arguments: default to 1440x960 physical screen pixels (matches 2x target 1:1 on Retina)
    int target_screen_px_w = 1440;
    int target_screen_px_h = 960;
    bool force_exact_points = false;
    int exact_points_w = 0, exact_points_h = 0;
    bool has_explicit_size = false;

    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--view") == 0 && i+1 < argc) {
            m_initial_view=argv[++i];
        } else if (strcmp(argv[i], "--adaptive") == 0) {
            m_snapshot_colors=true;
        } else if (strcmp(argv[i], "--ui-scale") == 0 && i+1 < argc) {
            m_snapshot_percent=atoi(argv[++i]);
        } else if (strcmp(argv[i], "--1x") == 0) {
            target_screen_px_w = 720;
            target_screen_px_h = 480;
            has_explicit_size = true;
        } else if (strcmp(argv[i], "--2x") == 0) {
            target_screen_px_w = 1440;
            target_screen_px_h = 960;
            has_explicit_size = true;
        } else if (strcmp(argv[i], "--3x") == 0) {
            target_screen_px_w = 2160;
            target_screen_px_h = 1440;
            has_explicit_size = true;
        } else if (strcmp(argv[i], "--4x") == 0) {
            target_screen_px_w = 2880;
            target_screen_px_h = 1920;
            has_explicit_size = true;
        } else if ((strcmp(argv[i], "--scale") == 0 || strcmp(argv[i], "-s") == 0) && i + 1 < argc) {
            float s = (float)atof(argv[++i]);
            if (s > 0.1f) {
                target_screen_px_w = (int)(720.0f * s + 0.5f);
                target_screen_px_h = (int)(480.0f * s + 0.5f);
                has_explicit_size = true;
            }
        } else if (strcmp(argv[i], "--pixels") == 0 && i + 1 < argc) {
            sscanf(argv[++i], "%dx%d", &target_screen_px_w, &target_screen_px_h);
            has_explicit_size = true;
        } else if (strcmp(argv[i], "--points") == 0 && i + 1 < argc) {
            force_exact_points = true;
            sscanf(argv[++i], "%dx%d", &exact_points_w, &exact_points_h);
            has_explicit_size = true;
        } else if (strcmp(argv[i], "--devices") == 0 || strcmp(argv[i], "--view-devices") == 0) {
            m_mode = Sidebar::Mode::Devices;
        } else if (strcmp(argv[i], "--sparklist") == 0 || strcmp(argv[i], "--modal") == 0 || strcmp(argv[i], "--view-modal") == 0) {
            m_show_modal = true;
        } else if (strcmp(argv[i], "--dump-png") == 0 && i + 1 < argc) {
            m_dump_png = argv[++i];
        }
    }

    if (!m_dump_png.empty()) m_experience.preferences=Preferences{};
    if (m_snapshot_colors) m_experience.preferences.coverflow_colors=true;
    if (Preferences::valid_scale(m_snapshot_percent)) m_experience.preferences.interface_percent=m_snapshot_percent;
    m_experience.name_draft=m_experience.preferences.display_name;
    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO)) {
        std::cerr << "SDL_Init failed: " << SDL_GetError() << "\n";
        return false;
    }

    // Auto-scale window to spacious Retina proportions (1280x850 pt) if not overridden by CLI
    if (!has_explicit_size && m_dump_png.empty()) {
        SDL_Rect usable;
        if (SDL_GetDisplayUsableBounds(SDL_GetPrimaryDisplay(), &usable)) {
            int desired_w = std::clamp((int)(usable.w * 0.85f), 960, 1280);
            int desired_h = std::clamp((int)(usable.h * 0.85f), 640, 850);
            force_exact_points = true;
            exact_points_w = desired_w;
            exact_points_h = desired_h;
        } else {
            force_exact_points = true;
            exact_points_w = 1280;
            exact_points_h = 850;
        }
    }

    // Hidden window if dumping PNG headless
    SDL_WindowFlags win_flags = SDL_WINDOW_BORDERLESS | SDL_WINDOW_HIGH_PIXEL_DENSITY | SDL_WINDOW_RESIZABLE | SDL_WINDOW_TRANSPARENT;
    if (!m_dump_png.empty()) {
        win_flags |= SDL_WINDOW_HIDDEN;
    }

    int initial_win_w = force_exact_points ? exact_points_w : (int)std::round((float)target_screen_px_w / 2.0f);
    int initial_win_h = force_exact_points ? exact_points_h : (int)std::round((float)target_screen_px_h / 2.0f);

    m_window = SDL_CreateWindow(
        "MusicOS Desktop",
        initial_win_w,
        initial_win_h,
        win_flags
    );
    if (!m_window) {
        std::cerr << "SDL_CreateWindow failed: " << SDL_GetError() << "\n";
        return false;
    }

    float density = 2.0f;
    if (m_window) {
        float d = SDL_GetWindowPixelDensity(m_window);
        if (d > 0.0f) density = d;
    }

    float initial_size=m_experience.preferences.interface_percent/100.f;
    SDL_SetWindowMinimumSize(m_window,int(720*initial_size),int(480*initial_size));
    if(force_exact_points){exact_points_w=std::max(exact_points_w,int(720*initial_size));exact_points_h=std::max(exact_points_h,int(480*initial_size));}
    if (force_exact_points) {
        if (m_window) SDL_SetWindowSize(m_window, exact_points_w, exact_points_h);
        target_screen_px_w = (int)std::round(exact_points_w * density);
        target_screen_px_h = (int)std::round(exact_points_h * density);
        m_logical_w = exact_points_w;
        m_logical_h = exact_points_h;
    } else {
        int points_w = (int)std::round((float)target_screen_px_w / density);
        int points_h = (int)std::round((float)target_screen_px_h / density);
        if (m_window) SDL_SetWindowSize(m_window, points_w, points_h);
        m_logical_w = points_w;
        m_logical_h = points_h;
    }

    if (m_window && m_dump_png.empty()) {
        SDL_SetWindowPosition(m_window, SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED);
    }

    m_renderer = SDL_CreateRenderer(m_window, NULL);
    if (!m_renderer) {
        std::cerr << "SDL_CreateRenderer failed: " << SDL_GetError() << "\n";
        return false;
    }

    int px_w = target_screen_px_w;
    int px_h = target_screen_px_h;
    if (m_dump_png.empty() && m_window) {
        SDL_GetWindowSizeInPixels(m_window, &px_w, &px_h);
        int win_w = 0, win_h = 0;
        SDL_GetWindowSize(m_window, &win_w, &win_h);
        if (win_w > 0 && win_h > 0) {
            m_logical_w = win_w;
            m_logical_h = win_h;
        }
    }
    m_render_w = px_w;
    m_render_h = px_h;
    float ui_scale=m_experience.preferences.interface_percent/100.f;
    m_logical_w=int(m_logical_w/ui_scale);
    m_logical_h=int(m_logical_h/ui_scale);
    m_scale = (float)m_render_w / (float)m_logical_w;

    SDL_SetRenderVSync(m_renderer, 1);
    SDL_SetRenderDrawColor(m_renderer,0,0,0,0);
    SDL_SetRenderLogicalPresentation(m_renderer,0,0,SDL_LOGICAL_PRESENTATION_DISABLED);

    m_texture = SDL_CreateTexture(
        m_renderer,
        SDL_PIXELFORMAT_ARGB8888,
        SDL_TEXTUREACCESS_STREAMING,
        m_render_w,
        m_render_h
    );
    if(m_texture)SDL_SetTextureBlendMode(m_texture,SDL_BLENDMODE_NONE);
    if (!m_texture) {
        std::cerr << "SDL_CreateTexture failed: " << SDL_GetError() << "\n";
        return false;
    }

    m_cairo_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    if (!m_cairo_surface) {
        std::cerr << "Failed to create Cairo surface\n";
        return false;
    }

    m_canvas = std::make_unique<Canvas>(m_cairo_surface, m_scale);
    m_canvas->color_mapper=[this](const Color& c){return m_experience.remap(c);};
    m_lib_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    m_dev_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    int modal_w = (int)(256.0f * m_scale + 0.5f);
    int modal_h = (int)(420.0f * m_scale + 0.5f);
    m_modal_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, modal_w, modal_h);
    m_blurred_bg_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);

    // Create UI Widgets
    m_cursor_default = SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_DEFAULT);
    m_cursor_pointer = SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_POINTER);
    m_resize_cursors[0]=SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_EW_RESIZE);
    m_resize_cursors[1]=SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_NS_RESIZE);
    m_resize_cursors[2]=SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_NWSE_RESIZE);
    m_resize_cursors[3]=SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_NESW_RESIZE);

    SDL_SetWindowHitTest(m_window, Application::hit_test_cb, this);
    SDL_RaiseWindow(m_window);
    SDL_StartTextInput(m_window);

    // Create UI Widgets
    m_traffic_lights = std::make_shared<TrafficLights>(Rect(18, 19, 96, 24));
    m_traffic_lights->on_close = [this]() { m_running = false; };
    m_traffic_lights->on_minimize = [this]() { SDL_MinimizeWindow(m_window); };
    m_traffic_lights->on_maximize = [this]() {
        SDL_WindowFlags flags = SDL_GetWindowFlags(m_window);
        if (flags & SDL_WINDOW_MAXIMIZED) {
            SDL_RestoreWindow(m_window);
        } else {
            SDL_MaximizeWindow(m_window);
        }
    };

    m_scope_selector = std::make_shared<ScopeSelector>(Rect(130, 7, 222, 48));
    m_scope_selector->mode = (m_mode == Sidebar::Mode::Devices) ? ScopeSelector::Mode::Devices : ScopeSelector::Mode::Library;
    m_scope_selector->on_mode_change = [this](ScopeSelector::Mode new_mode) {
        if (m_experience.editing_name && !m_experience.commit_name()) {m_scope_selector->mode=static_cast<ScopeSelector::Mode>(m_mode);return;}
        m_experience.editing_name=false;
        set_mode(static_cast<Sidebar::Mode>(new_mode));
    };

    m_now_playing = std::make_shared<NowPlayingPill>(Rect(368, 7, 334, 48));

    // Left Sidebar
    Rect sb_bounds = (m_mode == Sidebar::Mode::Devices) ? Rect(2, 100, 255, 378) : Rect(2, 64, 196, 414);
    m_sidebar = std::make_shared<Sidebar>(sb_bounds);
    m_sidebar->mode = m_mode;
    m_sidebar->on_create_sparklist_click = [this]() {
        m_show_modal = true;
    };

    // Album Grid (Library content)
    m_album_grid = std::make_shared<AlbumGrid>(Rect(200, 64, 518, 414));
    m_album_grid->library=&m_experience.library;
    m_album_grid->rebuild();

    // Devices Mode Content
    m_puffy_toggle = std::make_shared<PuffyToggle>(Rect(275, 112, 423, 53));
    m_puffy_toggle->selection = PuffyToggle::Selection::ManualManage;
    m_manual_sync_card = std::make_shared<ManualSyncCard>(Rect(273, 173, 428, 211));
    m_qualitizer_card = std::make_shared<QualitizerCard>(Rect(273, 386, 428, 84));

    // Initialize real-time audio playback stream
    AudioEngine::instance().init();

    m_experience.on_preferences_changed=[this]{preferences_changed();};
    m_experience.on_playback_changed=[this]{sync_demo_player();};
    m_now_playing->state.title="Choose a song";
    m_now_playing->state.artist="Demo library";
    m_now_playing->state.cover_path.clear();
    m_now_playing->state.progress=0;
    m_now_playing->play_morph.snap(0);
    m_now_playing->note_morph.snap(0);
    m_now_playing->on_play_pause=[this]{
        if(m_experience.sequence.ids.empty() && m_now_playing->state.cover_path.empty()) {m_experience.sequence.start(m_experience.library.albums.front().tracks);sync_demo_player();return;}
        AudioEngine::instance().set_playing(m_now_playing->state.is_playing);
    };
    m_now_playing->on_shuffle_toggle=[this]{m_experience.sequence.set_shuffle(m_now_playing->state.shuffle_on);};
    m_now_playing->on_seek=[](float progress){AudioEngine::instance().seek(progress);};
    m_now_playing->on_next=[this]{m_experience.advance(1);};
    m_now_playing->on_prev=[this]{m_experience.advance(-1);};
    m_album_grid->on_search_catalog=[](const std::string& query,int category,const std::string& genre){MusicSearchService::instance().search_async(query,category,genre);};
    m_album_grid->on_artist_select=[this](const std::string& id){
        m_experience.open(LibraryPage::Artist,id);
        update_back_label();
    };
    m_album_grid->on_album_select=[this](int index){
        const auto card=m_album_grid->cards.at(index);
        using Kind=AlbumGrid::AlbumCard::Kind;
        if(card.kind==Kind::Album){
            m_experience.open(LibraryPage::Album,card.id);
        }else if(card.kind==Kind::Artist)m_album_grid->on_artist_select(card.id);
        else if(card.kind==Kind::Song){
            std::vector<std::string> tracks;for(const auto& item:m_album_grid->cards)tracks.push_back(item.id);
            m_experience.sequence.start(std::move(tracks),false,card.id);sync_demo_player();
        }
        update_back_label();
    };
    m_album_grid->on_catalog_results=[this](const CatalogBatch& batch){
        m_experience.library.import_catalog(batch.records);
        m_album_grid->apply_catalog(batch);
        if(!batch.context.empty())m_experience.data_status=batch.error.empty()?(batch.records.empty()?"No catalog data available":""):batch.error;
        m_needs_redraw=true;
    };
    m_experience.on_catalog_lookup=[](const std::string& id,bool artist){MusicSearchService::instance().lookup_async(id,artist);};
    m_sidebar->on_back=[this]{m_experience.back();update_back_label();};
    m_sidebar->on_nav_change=[this](int category){
        m_experience.navigation.grid();
        m_experience.menu_open=false;
        m_album_grid->set_category(category);
        update_back_label();
    };
    m_sidebar->on_create_playlist_click=[this]{show_sparklist_modal(true);};
    // Existing playlist prototypes continue to use a demo sequence.
    m_sidebar->on_playlist_select=[this](int index){
        auto& album=m_experience.library.albums.at(size_t(index)%m_experience.library.albums.size());
        m_experience.sequence.start(album.tracks);sync_demo_player();
    };

    // Qualitizer & Sync dynamic storage recalculation
    m_qualitizer_card->on_rate_change = [this](int rate) {
        if (m_sidebar) m_sidebar->set_storage_rate(rate);
        m_dev_dirty = true;
        m_needs_redraw = true;
    };
    m_manual_sync_card->on_sync_change = [this](int count) {
        if (m_sidebar) m_sidebar->set_sync_count(count);
        m_dev_dirty = true;
        m_needs_redraw = true;
    };

    // Modal
    m_modal = std::make_shared<SparklistModal>(Rect(232, 30, 256, 420));
    m_modal->on_close = [this]() { show_sparklist_modal(false); };
    m_modal->on_song_chosen = [this](const SparklistModal::SparkSong& song) {
        m_now_playing->state.title = song.title;
        m_now_playing->state.artist = song.artist + " (Sparklist Radio)";
        m_now_playing->state.cover_path = song.cover_path;
        m_now_playing->state.progress = 0.0f;
        m_now_playing->state.is_playing = true;
        m_now_playing->play_pop.snap(0.85f);
        m_now_playing->play_pop.set_target(1.0f);
        m_now_playing->play_morph.set_target(1.0f);
        AudioEngine::instance().set_vibe(song.vibe_idx);
        AudioEngine::instance().play();
        show_sparklist_modal(false);
        m_needs_redraw = true;
    };
    m_modal->on_choose_for_me = [this]() {
        m_now_playing->state.title = "Sparklist Mix: Neo-Tokyo Dream";
        m_now_playing->state.artist = "MusicOS AI Radio Station";
        m_now_playing->state.cover_path = "assets/covers/Beach.png";
        m_now_playing->state.progress = 0.0f;
        m_now_playing->state.is_playing = true;
        m_now_playing->play_pop.snap(0.85f);
        m_now_playing->play_pop.set_target(1.0f);
        m_now_playing->play_morph.set_target(1.0f);
        AudioEngine::instance().set_vibe(3);
        AudioEngine::instance().play();
        show_sparklist_modal(false);
        m_needs_redraw = true;
    };

    m_sidebar->on_create_sparklist_click = [this]() {
        show_sparklist_modal(true);
    };

    if (m_show_modal) {
        m_modal_scale.snap(1.0f);
        m_modal_alpha.snap(1.0f);
    }
    if (m_mode == Sidebar::Mode::Devices) {
        m_view_transition.snap(1.0f);
        m_puffy_toggle->anim_x.snap(208.0f);
        m_qualitizer_card->anim_x.snap(0.0f);
        m_manual_sync_card->tab_bar->anim_x.snap(0.0f);
    } else {
        m_view_transition.snap(0.0f);
    }

    if(m_initial_view=="album")m_experience.open(LibraryPage::Album,m_experience.library.albums.front().id);
    if(m_initial_view=="artist")m_experience.open(LibraryPage::Artist,"m83");
    if(m_initial_view=="settings") {m_mode=Sidebar::Mode::Settings;m_experience.settings=true;m_scope_selector->mode=ScopeSelector::Mode::Settings;}
    m_sidebar->display_name=m_experience.preferences.display_name;
    update_back_label();
    update_layout();
    m_experience.update(1,m_now_playing->state.cover_path,m_mode==Sidebar::Mode::Library);

    // Pre-warm and pre-cache surfaces so interaction has 0ms latency and 0 dropped frames
    update_lib_surface();
    update_dev_surface();
    update_modal_surface();

    return true;
}

void Application::set_mode(Sidebar::Mode mode) {
    if(m_mode==Sidebar::Mode::Settings && mode!=Sidebar::Mode::Settings && m_experience.editing_name) {
        if(!m_experience.commit_name())return;
        m_experience.editing_name=false;
    }
    m_mode=mode;
    m_experience.settings=mode==Sidebar::Mode::Settings;
    m_experience.reveal();
    m_experience.menu_open=false;
    m_scope_selector->mode=static_cast<ScopeSelector::Mode>(mode);
    m_scope_selector->scope_morph.set_target(mode==Sidebar::Mode::Devices?1.f:0.f);
    m_view_transition.snap(mode==Sidebar::Mode::Devices?1.f:0.f);
    update_layout();
    m_modal_bg_cached=false;
    m_needs_redraw=true;
}
void Application::update_back_label() {
    auto& nav=m_experience.navigation;
    m_sidebar->back_label=nav.current.page==LibraryPage::Grid?"":
        (!nav.history.empty() && nav.history.back().page==LibraryPage::Artist?"Back to Artist":(!nav.history.empty() && nav.history.back().page==LibraryPage::Album?"Back to Album":(m_album_grid->category==1?"Back to Artists":m_album_grid->category==2?"Back to Genres":m_album_grid->category==3?"Back to Songs":"Back to Albums")));
}
void Application::sync_demo_player() {
    const auto id=m_experience.sequence.current();
    auto t=m_experience.library.tracks.find(id);if(t==m_experience.library.tracks.end())return;
    auto a=m_experience.library.album(t->second.album_id);
    auto& state=m_now_playing->state;
    state.title=t->second.title;state.artist=a->artist+" · Demo audio";state.cover_path=a->cover;
    state.progress=0;state.is_playing=true;
    m_album_grid->playing_track=id;
    state.shuffle_on=m_experience.sequence.shuffled;
    m_now_playing->shuffle_morph.set_target(state.shuffle_on?1.f:0.f);
    m_now_playing->play_morph.set_target(1);
    m_now_playing->note_morph.set_target(1);
    AudioEngine::instance().set_vibe(int(m_experience.sequence.position%4));
    AudioEngine::instance().seek(0);AudioEngine::instance().play();
    m_needs_redraw=true;
}
void Application::preferences_changed() {
    m_sidebar->display_name=m_experience.preferences.display_name;
    float size=m_experience.preferences.interface_percent/100.f;
    SDL_SetWindowMinimumSize(m_window,int(720*size),int(480*size));
    int w=0,h=0;SDL_GetWindowSize(m_window,&w,&h);
    if(w<int(720*size) || h<int(480*size))SDL_SetWindowSize(m_window,std::max(w,int(720*size)),std::max(h,int(480*size)));
    int pw=0,ph=0;SDL_GetWindowSizeInPixels(m_window,&pw,&ph);resize_buffers(pw,ph);
    m_lib_dirty=m_dev_dirty=m_modal_dirty=true;m_modal_bg_cached=false;m_needs_redraw=true;
}
void Application::render_library() {
    update_back_label();
    m_sidebar->mode=Sidebar::Mode::Library;
    m_sidebar->render(*m_canvas);
    if(m_experience.navigation.current.page==LibraryPage::Grid)m_album_grid->render(*m_canvas);
    else m_experience.render(*m_canvas,m_album_grid->bounds);
}

void Application::show_sparklist_modal(bool show) {
    m_show_modal = show;
    if (show) {
        if (m_modal) {
            m_modal->sparkle_morph.snap(0.0f);
            m_modal->sparkle_morph.set_target(1.0f);
        }
        update_modal_surface();
        m_modal_bg_cached = false; // snapshot and blur on frame 0
        m_modal_scale.snap(0.88f);
        m_modal_scale.set_target(1.0f);
        m_modal_alpha.snap(0.0f);
        m_modal_alpha.set_target(1.0f);
    } else {
        if (m_modal) {
            m_modal->sparkle_morph.set_target(0.0f);
        }
        m_modal_scale.set_target(0.90f);
        m_modal_alpha.set_target(0.0f);
    }
    m_needs_redraw = true;
}

void Application::save_screenshot(const std::string& path) {
    render_frame();
    cairo_status_t status = cairo_surface_write_to_png(m_cairo_surface, path.c_str());
    if (status != CAIRO_STATUS_SUCCESS) {
        std::cerr << "save_screenshot error: " << cairo_status_to_string(status) << " for " << path << "\n";
    }
}

void Application::update_layout() {
    if (m_traffic_lights) {
        m_traffic_lights->bounds = Rect(18, 19, 96, 24);
    }
    if (m_scope_selector) {
        m_scope_selector->bounds = Rect(130, 7, 222, 48);
    }
    if (m_now_playing) {
        float pill_w = std::clamp((float)m_logical_w - 386.0f,334.f,560.f);
        float pill_x = m_logical_w - pill_w - 18.f;
        m_now_playing->bounds = Rect(pill_x, 7.0f, pill_w, 48.0f);
    }

    if (m_sidebar) {
        if (m_mode == Sidebar::Mode::Library) {
            m_sidebar->bounds = Rect(2, 64, 196, (float)m_logical_h - 66.0f);
        } else {
            m_sidebar->bounds = Rect(2, 100, 255, (float)m_logical_h - 102.0f);
        }
    }

    if (m_album_grid) {
        m_album_grid->bounds = Rect(200, 64, (float)m_logical_w - 202.0f, (float)m_logical_h - 66.0f);
    }

    float dev_x = 259.0f;
    float dev_y = 100.0f;
    float dev_w = (float)m_logical_w - dev_x - 5.0f;
    float dev_h = (float)m_logical_h - dev_y - 2.0f;
    float cw = dev_w - 28.0f;

    if (m_logical_w == 720 && m_logical_h == 480) {
        if (m_puffy_toggle) m_puffy_toggle->bounds = Rect(275, 112, 423, 53);
        if (m_manual_sync_card) {
            m_manual_sync_card->bounds = Rect(273, 173, 428, 211);
            if (m_manual_sync_card->tab_bar) {
                m_manual_sync_card->tab_bar->bounds = Rect(281, 181, 412, 32);
            }
        }
        if (m_qualitizer_card) m_qualitizer_card->bounds = Rect(273, 386, 428, 84);
    } else {
        if (m_puffy_toggle) {
            m_puffy_toggle->bounds = Rect(dev_x + 16.0f, dev_y + 12.0f, cw - 5.0f, 53.0f);
        }
        if (m_manual_sync_card) {
            m_manual_sync_card->bounds = Rect(dev_x + 14.0f, dev_y + 73.0f, cw, dev_h - 167.0f);
            if (m_manual_sync_card->tab_bar) {
                m_manual_sync_card->tab_bar->bounds = Rect(m_manual_sync_card->bounds.x + 8.0f, m_manual_sync_card->bounds.y + 8.0f, m_manual_sync_card->bounds.w - 16.0f, 32.0f);
            }
        }
        if (m_qualitizer_card) {
            m_qualitizer_card->bounds = Rect(dev_x + 14.0f, dev_y + dev_h - 92.0f, cw, 84.0f);
        }
    }

    if (m_modal) {
        m_modal->bounds = Rect(
            ((float)m_logical_w - 256.0f) * 0.5f,
            ((float)m_logical_h - 420.0f) * 0.5f,
            256.0f,
            420.0f
        );
    }
}

void Application::toggle_maximize() {
    if (!m_window) return;
    SDL_WindowFlags flags = SDL_GetWindowFlags(m_window);
    if (flags & SDL_WINDOW_MAXIMIZED) {
        SDL_RestoreWindow(m_window);
    } else {
        SDL_MaximizeWindow(m_window);
    }
}

void Application::set_window_size_points(int points_w, int points_h) {
    if (!m_window) return;
    SDL_SetWindowSize(m_window, points_w, points_h);
    int pw = 0, ph = 0;
    SDL_GetWindowSizeInPixels(m_window, &pw, &ph);
    if (pw > 0 && ph > 0) {
        resize_buffers(pw, ph);
    }
}

void Application::set_screen_pixel_size(int target_px_w, int target_px_h) {
    if (!m_window) return;
    float density = SDL_GetWindowPixelDensity(m_window);
    if (density <= 0.0f) density = 1.0f;
    int points_w = (int)std::round((float)target_px_w / density);
    int points_h = (int)std::round((float)target_px_h / density);
    set_window_size_points(points_w, points_h);
}

void Application::resize_buffers(int new_render_w, int new_render_h) {
    if (new_render_w <= 0 || new_render_h <= 0) return;

    int win_w = 0, win_h = 0;
    if (m_window) {
        SDL_GetWindowSize(m_window, &win_w, &win_h);
    }
    if (win_w <= 0) win_w = (int)std::round((float)new_render_w / m_scale);
    if (win_h <= 0) win_h = (int)std::round((float)new_render_h / m_scale);

    float ui_scale=m_experience.preferences.interface_percent/100.f;
    win_w=int(win_w/ui_scale);win_h=int(win_h/ui_scale);
    if (new_render_w == m_render_w && new_render_h == m_render_h &&
        win_w == m_logical_w && win_h == m_logical_h && m_canvas != nullptr) return;

    m_logical_w = win_w;
    m_logical_h = win_h;
    m_render_w = new_render_w;
    m_render_h = new_render_h;
    m_scale = (float)m_render_w / (float)m_logical_w;

    if (m_renderer) {
        SDL_SetRenderLogicalPresentation(m_renderer,0,0,SDL_LOGICAL_PRESENTATION_DISABLED);
    }

    if (m_cairo_surface) { cairo_surface_destroy(m_cairo_surface); m_cairo_surface = nullptr; }
    if (m_lib_surface) { cairo_surface_destroy(m_lib_surface); m_lib_surface = nullptr; }
    if (m_dev_surface) { cairo_surface_destroy(m_dev_surface); m_dev_surface = nullptr; }
    if (m_modal_surface) { cairo_surface_destroy(m_modal_surface); m_modal_surface = nullptr; }
    if (m_blurred_bg_surface) { cairo_surface_destroy(m_blurred_bg_surface); m_blurred_bg_surface = nullptr; }
    if (m_texture) { SDL_DestroyTexture(m_texture); m_texture = nullptr; }

    m_cairo_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    if (!m_canvas) {
        m_canvas = std::make_unique<Canvas>(m_cairo_surface, m_scale);
    } else {
        m_canvas->set_surface(m_cairo_surface, m_scale);
    }

    if (m_renderer) {
        m_texture = SDL_CreateTexture(
            m_renderer,
            SDL_PIXELFORMAT_ARGB8888,
            SDL_TEXTUREACCESS_STREAMING,
            m_render_w,
            m_render_h
        );
    }

    m_lib_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    m_dev_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    int modal_w = (int)(256.0f * m_scale + 0.5f);
    int modal_h = (int)(420.0f * m_scale + 0.5f);
    m_modal_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, modal_w, modal_h);
    m_blurred_bg_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);

    if(m_texture)SDL_SetTextureBlendMode(m_texture,SDL_BLENDMODE_NONE);
    update_layout();
    m_lib_dirty=m_dev_dirty=m_modal_dirty=true;
    m_modal_bg_cached = false;
    m_needs_redraw = true;
}

static void box_blur_h(const uint32_t* src, uint32_t* dst, int w, int h, int r) {
    if (w <= 0 || h <= 0 || r <= 0) return;
    float iarr = 1.0f / (r + r + 1);
    for (int y = 0; y < h; ++y) {
        int ti = y * w;
        int li = ti;
        int ri = ti + r;
        int row_end = (y + 1) * w - 1;
        uint32_t fv = src[ti];
        uint32_t lv = src[row_end];
        int val_r = ((fv >> 16) & 0xFF) * (r + 1);
        int val_g = ((fv >> 8) & 0xFF) * (r + 1);
        int val_b = (fv & 0xFF) * (r + 1);
        int val_a = ((fv >> 24) & 0xFF) * (r + 1);
        for (int j = 0; j < r; ++j) {
            uint32_t cur = src[std::min(ti + j, row_end)];
            val_r += (cur >> 16) & 0xFF;
            val_g += (cur >> 8) & 0xFF;
            val_b += cur & 0xFF;
            val_a += (cur >> 24) & 0xFF;
        }
        for (int j = 0; j <= r && ti <= row_end; ++j) {
            uint32_t cur = src[std::min(ri++, row_end)];
            val_r += ((cur >> 16) & 0xFF) - ((fv >> 16) & 0xFF);
            val_g += ((cur >> 8) & 0xFF) - ((fv >> 8) & 0xFF);
            val_b += (cur & 0xFF) - (fv & 0xFF);
            val_a += ((cur >> 24) & 0xFF) - ((fv >> 24) & 0xFF);
            dst[ti++] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                        ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                        ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                        (uint32_t)(val_b * iarr + 0.5f);
        }
        for (int j = r + 1; j < w - r && ti <= row_end; ++j) {
            uint32_t cur_r = src[std::min(ri++, row_end)];
            uint32_t cur_l = src[std::min(li++, row_end)];
            val_r += ((cur_r >> 16) & 0xFF) - ((cur_l >> 16) & 0xFF);
            val_g += ((cur_r >> 8) & 0xFF) - ((cur_l >> 8) & 0xFF);
            val_b += (cur_r & 0xFF) - (cur_l & 0xFF);
            val_a += ((cur_r >> 24) & 0xFF) - ((cur_l >> 24) & 0xFF);
            dst[ti++] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                        ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                        ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                        (uint32_t)(val_b * iarr + 0.5f);
        }
        for (int j = std::max(0, w - r); j < w && ti <= row_end; ++j) {
            uint32_t cur = src[std::min(li++, row_end)];
            val_r += ((lv >> 16) & 0xFF) - ((cur >> 16) & 0xFF);
            val_g += ((lv >> 8) & 0xFF) - ((cur >> 8) & 0xFF);
            val_b += (lv & 0xFF) - (cur & 0xFF);
            val_a += ((lv >> 24) & 0xFF) - ((cur >> 24) & 0xFF);
            dst[ti++] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                        ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                        ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                        (uint32_t)(val_b * iarr + 0.5f);
        }
    }
}

static void box_blur_v(const uint32_t* src, uint32_t* dst, int w, int h, int r) {
    if (w <= 0 || h <= 0 || r <= 0) return;
    float iarr = 1.0f / (r + r + 1);
    int total_pixels = w * h;
    for (int x = 0; x < w; ++x) {
        int ti = x;
        int li = ti;
        int ri = ti + r * w;
        uint32_t fv = 0xFF80D3FF; // blend top edge against window background #80D3FF
        int max_y_idx = x + (h - 1) * w;
        uint32_t lv = (max_y_idx < total_pixels) ? src[max_y_idx] : src[x];
        int val_r = ((fv >> 16) & 0xFF) * (r + 1);
        int val_g = ((fv >> 8) & 0xFF) * (r + 1);
        int val_b = (fv & 0xFF) * (r + 1);
        int val_a = ((fv >> 24) & 0xFF) * (r + 1);
        for (int j = 0; j < r; ++j) {
            int cur_idx = std::min(ti + j * w, max_y_idx);
            uint32_t cur = src[cur_idx];
            val_r += (cur >> 16) & 0xFF;
            val_g += (cur >> 8) & 0xFF;
            val_b += cur & 0xFF;
            val_a += (cur >> 24) & 0xFF;
        }
        for (int j = 0; j <= r && ti <= max_y_idx; ++j) {
            int cur_idx = std::min(ri, max_y_idx);
            ri += w;
            uint32_t cur = src[cur_idx];
            val_r += ((cur >> 16) & 0xFF) - ((fv >> 16) & 0xFF);
            val_g += ((cur >> 8) & 0xFF) - ((fv >> 8) & 0xFF);
            val_b += (cur & 0xFF) - (fv & 0xFF);
            val_a += ((cur >> 24) & 0xFF) - ((fv >> 24) & 0xFF);
            dst[ti] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                      ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                      ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                      (uint32_t)(val_b * iarr + 0.5f);
            ti += w;
        }
        for (int j = r + 1; j < h - r && ti <= max_y_idx; ++j) {
            int cur_r_idx = std::min(ri, max_y_idx);
            ri += w;
            int cur_l_idx = std::min(li, max_y_idx);
            li += w;
            uint32_t cur_r = src[cur_r_idx];
            uint32_t cur_l = src[cur_l_idx];
            val_r += ((cur_r >> 16) & 0xFF) - ((cur_l >> 16) & 0xFF);
            val_g += ((cur_r >> 8) & 0xFF) - ((cur_l >> 8) & 0xFF);
            val_b += (cur_r & 0xFF) - (cur_l & 0xFF);
            val_a += ((cur_r >> 24) & 0xFF) - ((cur_l >> 24) & 0xFF);
            dst[ti] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                      ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                      ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                      (uint32_t)(val_b * iarr + 0.5f);
            ti += w;
        }
        for (int j = std::max(0, h - r); j < h && ti <= max_y_idx; ++j) {
            int cur_idx = std::min(li, max_y_idx);
            li += w;
            uint32_t cur = src[cur_idx];
            val_r += ((lv >> 16) & 0xFF) - ((cur >> 16) & 0xFF);
            val_g += ((lv >> 8) & 0xFF) - ((cur >> 8) & 0xFF);
            val_b += (lv & 0xFF) - (cur & 0xFF);
            val_a += ((lv >> 24) & 0xFF) - ((cur >> 24) & 0xFF);
            dst[ti] = ((uint32_t)(val_a * iarr + 0.5f) << 24) |
                      ((uint32_t)(val_r * iarr + 0.5f) << 16) |
                      ((uint32_t)(val_g * iarr + 0.5f) << 8) |
                      (uint32_t)(val_b * iarr + 0.5f);
            ti += w;
        }
    }
}

static void fast_gaussian_blur(uint32_t* pixels, int w, int h, int radius) {
    if (radius <= 0) return;
    std::vector<uint32_t> temp(w * h);
    int r = std::max(1, radius);
    for (int p = 0; p < 3; ++p) {
        box_blur_h(pixels, temp.data(), w, h, r);
        box_blur_v(temp.data(), pixels, w, h, r);
    }
}


void Application::update_lib_surface() {
    if (!m_lib_surface || !m_canvas) return;
    cairo_t* cr_raw = cairo_create(m_lib_surface);
    cairo_set_operator(cr_raw, CAIRO_OPERATOR_CLEAR);
    cairo_paint(cr_raw);
    cairo_destroy(cr_raw);

    m_canvas->set_surface(m_lib_surface, m_scale);
    m_album_grid->show_search_bar = true;
    m_sidebar->mode = Sidebar::Mode::Library;
    render_library();
    cairo_surface_flush(m_lib_surface);
    m_canvas->set_surface(m_cairo_surface, m_scale);
    m_lib_dirty = false;
}

void Application::update_dev_surface() {
    if (!m_dev_surface || !m_canvas) return;
    cairo_t* cr_raw = cairo_create(m_dev_surface);
    cairo_set_operator(cr_raw, CAIRO_OPERATOR_CLEAR);
    cairo_paint(cr_raw);
    cairo_destroy(cr_raw);

    m_canvas->set_surface(m_dev_surface, m_scale);
    Rect dev_tab_bar(2, 65, (float)m_logical_w - 4.0f, 32.0f);
    m_canvas->fill_rounded_rect(dev_tab_bar, 16.0f, Color::from_hex(0xD6EDFD));
    m_canvas->draw_text("Henry Music 01", Point(dev_tab_bar.x + 8.0f, dev_tab_bar.y + 9.0f), 14.0f, Color::Black, FontWeight::Medium);
    m_canvas->draw_svg("assets/icons/sketch/tinyMinimize.svg", Rect(dev_tab_bar.x + 116.0f, dev_tab_bar.y + 7.0f, 16.0f, 16.0f));

    float opt_x[3] = { dev_tab_bar.w - 207.0f, dev_tab_bar.w - 141.0f, dev_tab_bar.w - 75.0f };
    for (int i = 0; i < 3; ++i) {
        Rect opt_btn(dev_tab_bar.x + opt_x[i], dev_tab_bar.y + 4.0f, 64.0f, 24.0f);
        m_canvas->fill_rounded_rect(opt_btn, 4.0f, Color::from_hex(0x0034FF));
        m_canvas->draw_text("Option", Point(opt_btn.x + 6.0f, opt_btn.y + 4.0f), 16.0f, Color::White, FontWeight::Medium, TextAlign::Left);
    }

    m_sidebar->mode = Sidebar::Mode::Devices;
    m_sidebar->render(*m_canvas);

    Rect dev_content(259, 100, (float)m_logical_w - 264.0f, (float)m_logical_h - 102.0f);
    m_canvas->fill_rounded_rect(dev_content, 16.0f, Color::from_hex(0x00AEFF), 0.6f);
    m_canvas->stroke_rounded_rect(dev_content, 16.0f, Color::from_hex(0x0018FF), 1.0f, 0.6f);

    m_puffy_toggle->render(*m_canvas);
    m_manual_sync_card->render(*m_canvas);
    m_qualitizer_card->render(*m_canvas);
    cairo_surface_flush(m_dev_surface);
    m_canvas->set_surface(m_cairo_surface, m_scale);
    m_dev_dirty = false;
}

void Application::update_modal_surface() {
    if (!m_modal_surface || !m_canvas) return;
    cairo_t* cr_raw = cairo_create(m_modal_surface);
    cairo_set_operator(cr_raw, CAIRO_OPERATOR_CLEAR);
    cairo_paint(cr_raw);
    cairo_destroy(cr_raw);

    m_canvas->set_surface(m_modal_surface, m_scale);
    Rect saved = m_modal->bounds;
    m_modal->bounds = Rect(0, 0, 256, 420);
    m_modal->render(*m_canvas);
    m_modal->bounds = saved;
    cairo_surface_flush(m_modal_surface);
    m_canvas->set_surface(m_cairo_surface, m_scale);
    m_modal_dirty = false;
}

void Application::update_blur_surface() {
    if (!m_blurred_bg_surface) {
        m_blurred_bg_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, m_render_w, m_render_h);
    }
    cairo_surface_flush(m_cairo_surface);
    uint32_t* src_data = (uint32_t*)cairo_image_surface_get_data(m_cairo_surface);
    uint32_t* dst_data = (uint32_t*)cairo_image_surface_get_data(m_blurred_bg_surface);
    memcpy(dst_data, src_data, m_render_w * m_render_h * sizeof(uint32_t));
    fast_gaussian_blur(dst_data, m_render_w, m_render_h, (int)(15.5f * m_scale + 0.5f));
    cairo_surface_mark_dirty(m_blurred_bg_surface);
    m_modal_bg_cached = true;
}

void Application::render_frame() {
    cairo_t* cr = m_canvas->cr();
    cairo_save(cr);
    cairo_set_operator(cr, CAIRO_OPERATOR_CLEAR);
    cairo_paint(cr);
    cairo_restore(cr);

    // 1. Clear base background: #80D3FF
    m_canvas->fill_rect(Rect(0, 0, m_logical_w, m_logical_h), Color::from_hex(0x80D3FF));

    // 2. Render Views with zero-allocation 120 FPS hardware blits
    bool in_transition = m_view_transition.is_animating();
    float vt = m_view_transition.val; // 0.0 = Library, 1.0 = Devices

    if (in_transition) {
        float alpha_lib = std::max(0.0f, std::min(1.0f, 1.0f - vt));
        float alpha_dev = std::max(0.0f, std::min(1.0f, vt));

        cairo_surface_flush(m_lib_surface);
        cairo_surface_flush(m_dev_surface);

        m_canvas->push_clip_rect(Rect(0, 62.0f, (float)m_logical_w, (float)m_logical_h - 62.0f));

        if (alpha_lib > 0.001f) {
            cairo_save(cr);
            cairo_translate(cr, -16.0f * (1.0f - alpha_lib), 0.0f);
            cairo_surface_set_device_scale(m_lib_surface, m_scale, m_scale);
            cairo_set_source_surface(cr, m_lib_surface, 0, 0);
            cairo_paint_with_alpha(cr, alpha_lib);
            cairo_restore(cr);
        }

        if (alpha_dev > 0.001f) {
            cairo_save(cr);
            cairo_translate(cr, 16.0f * (1.0f - alpha_dev), 0.0f);
            cairo_surface_set_device_scale(m_dev_surface, m_scale, m_scale);
            cairo_set_source_surface(cr, m_dev_surface, 0, 0);
            cairo_paint_with_alpha(cr, alpha_dev);
            cairo_restore(cr);
        }

        m_canvas->pop_clip();
    } else {
        if (m_mode == Sidebar::Mode::Library) {
            m_album_grid->show_search_bar = (m_modal_alpha.val < 0.5f);
            render_library();
        } else if(m_mode==Sidebar::Mode::Settings) {
            m_experience.render(*m_canvas,Rect(2,64,m_logical_w-4,m_logical_h-66));
        } else {
            // Secondary Header Frame 5: (2, 65, m_logical_w - 4, 32), fill #D6EDFD
            Rect dev_tab_bar(2, 65, (float)m_logical_w - 4.0f, 32.0f);
            m_canvas->fill_rounded_rect(dev_tab_bar, 16.0f, Color::from_hex(0xD6EDFD));
            m_canvas->draw_text("Henry Music 01", Point(dev_tab_bar.x + 8.0f, dev_tab_bar.y + 9.0f), 14.0f, Color::Black, FontWeight::Medium);
            m_canvas->draw_svg("assets/icons/sketch/tinyMinimize.svg", Rect(dev_tab_bar.x + 116.0f, dev_tab_bar.y + 7.0f, 16.0f, 16.0f));

            // 3 option buttons on right
            float opt_x[3] = { dev_tab_bar.w - 207.0f, dev_tab_bar.w - 141.0f, dev_tab_bar.w - 75.0f };
            for (int i = 0; i < 3; ++i) {
                Rect opt_btn(dev_tab_bar.x + opt_x[i], dev_tab_bar.y + 4.0f, 64.0f, 24.0f);
                m_canvas->fill_rounded_rect(opt_btn, 4.0f, Color::from_hex(0x0034FF));
                m_canvas->draw_text("Option", Point(opt_btn.x + 6.0f, opt_btn.y + 4.0f), 16.0f, Color::White, FontWeight::Medium, TextAlign::Left);
            }

            m_sidebar->mode = Sidebar::Mode::Devices;
            m_sidebar->render(*m_canvas);

            Rect dev_content(259, 100, (float)m_logical_w - 264.0f, (float)m_logical_h - 102.0f);
            m_canvas->fill_rounded_rect(dev_content, 16.0f, Color::from_hex(0x00AEFF), 0.6f);
            m_canvas->stroke_rounded_rect(dev_content, 16.0f, Color::from_hex(0x0018FF), 1.0f, 0.6f);

            m_puffy_toggle->render(*m_canvas);
            m_manual_sync_card->render(*m_canvas);
            m_qualitizer_card->render(*m_canvas);
        }
    }

    // 3. Render Pinned Toolbar on top
    m_canvas->fill_rounded_rect(Rect(0, 0, m_logical_w, 62.0f), 16.0f, Color::from_hex(0x00BAFF));
    m_traffic_lights->render(*m_canvas);
    m_scope_selector->render(*m_canvas);
    m_now_playing->render(*m_canvas);
    m_scope_selector->render_menu(*m_canvas);

    // 4. Render Modal if visible (with silky blur fade & snappy spring pop!)
    if (m_modal_alpha.val > 0.005f) {
        if (!m_modal_bg_cached) {
            update_blur_surface();
        }

        float ma = std::max(0.0f, std::min(1.0f, m_modal_alpha.val));
        float ms = std::max(0.5f, m_modal_scale.val);

        cairo_surface_flush(m_blurred_bg_surface);
        cairo_save(cr);
        cairo_surface_set_device_scale(m_blurred_bg_surface, m_scale, m_scale);
        cairo_set_source_surface(cr, m_blurred_bg_surface, 0, 0);
        cairo_paint_with_alpha(cr, ma);
        cairo_restore(cr);

        if (m_modal_dirty) {
            update_modal_surface();
        }

        float mcx = m_modal->bounds.x + m_modal->bounds.w * 0.5f;
        float mcy = m_modal->bounds.y + m_modal->bounds.h * 0.5f;

        if (ma >= 0.999f && std::abs(ms - 1.0f) < 0.001f) {
            m_modal->render(*m_canvas);
        } else {
            cairo_save(cr);
            cairo_translate(cr, mcx, mcy);
            cairo_scale(cr, ms, ms);
            cairo_surface_set_device_scale(m_modal_surface, m_scale, m_scale);
            cairo_set_source_surface(cr, m_modal_surface, -m_modal->bounds.w * 0.5f, -m_modal->bounds.h * 0.5f);
            cairo_paint_with_alpha(cr, ma);
            cairo_restore(cr);
        }
    }

    m_canvas->mask_rounded_rect(Rect(0, 0, m_logical_w, m_logical_h), 16.0f, 0.6f);

    // 5. Flush Cairo and upload to SDL3 streaming texture
    cairo_surface_flush(m_cairo_surface);
    unsigned char* cairo_data = cairo_image_surface_get_data(m_cairo_surface);
    int cairo_stride = cairo_image_surface_get_stride(m_cairo_surface);

    SDL_UpdateTexture(m_texture, NULL, cairo_data, cairo_stride);

    SDL_RenderClear(m_renderer);
    SDL_RenderTexture(m_renderer, m_texture, NULL, NULL);
    SDL_RenderPresent(m_renderer);
}

void Application::handle_event(const SDL_Event& raw_event) {
    m_needs_redraw = true;
    SDL_Event event = raw_event;
    // SDL mouse coordinates are window points. Invert the same root transform
    // used by Cairo, independently of Retina density and renderer target state.
    int input_w=0,input_h=0;SDL_GetWindowSize(m_window,&input_w,&input_h);
    float input_x=input_w>0?float(m_logical_w)/input_w:1.f;
    float input_y=input_h>0?float(m_logical_h)/input_h:1.f;
    if(event.type==SDL_EVENT_MOUSE_BUTTON_DOWN || event.type==SDL_EVENT_MOUSE_BUTTON_UP){event.button.x*=input_x;event.button.y*=input_y;}
    if(event.type==SDL_EVENT_MOUSE_MOTION){event.motion.x*=input_x;event.motion.y*=input_y;event.motion.xrel*=input_x;event.motion.yrel*=input_y;}

    if(event.type==SDL_EVENT_MOUSE_BUTTON_DOWN && event.button.button==SDL_BUTTON_LEFT){
        m_resize_edges=resize_edges(raw_event.button.x,raw_event.button.y);
        if(m_resize_edges){
            SDL_GetWindowPosition(m_window,&m_resize_x,&m_resize_y);SDL_GetWindowSize(m_window,&m_resize_w,&m_resize_h);
            SDL_GetGlobalMouseState(&m_resize_mouse_x,&m_resize_mouse_y);SDL_CaptureMouse(true);return;
        }
    }
    if(m_resize_edges && event.type==SDL_EVENT_MOUSE_MOTION){
        float mx,my;SDL_GetGlobalMouseState(&mx,&my);int dx=int(mx-m_resize_mouse_x),dy=int(my-m_resize_mouse_y);
        int minw,minh;SDL_GetWindowMinimumSize(m_window,&minw,&minh);
        int width=std::max(minw,m_resize_w+((m_resize_edges&1)?-dx:(m_resize_edges&2)?dx:0));
        int height=std::max(minh,m_resize_h+((m_resize_edges&4)?-dy:(m_resize_edges&8)?dy:0));
        int nx=m_resize_x+((m_resize_edges&1)?m_resize_w-width:0),ny=m_resize_y+((m_resize_edges&4)?m_resize_h-height:0);
        SDL_SetWindowPosition(m_window,nx,ny);SDL_SetWindowSize(m_window,width,height);
        int pw,ph;SDL_GetWindowSizeInPixels(m_window,&pw,&ph);if(pw!=m_render_w || ph!=m_render_h)resize_buffers(pw,ph);return;
    }
    if(event.type==SDL_EVENT_MOUSE_BUTTON_UP && m_resize_edges){m_resize_edges=0;SDL_CaptureMouse(false);return;}
    if (event.type == SDL_EVENT_QUIT) {
        m_running = false;
        return;
    }

    if (event.type == SDL_EVENT_USER) {
        if (m_album_grid) {
            m_album_grid->ingest_pending_results();
            m_lib_dirty = true;
            m_needs_redraw = true;
        }
        return;
    }

    if (event.type == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED || event.type == SDL_EVENT_WINDOW_RESIZED) {
        int pw = 0, ph = 0;
        SDL_GetWindowSizeInPixels(m_window, &pw, &ph);
        if (pw > 0 && ph > 0 && (pw != m_render_w || ph != m_render_h)) {
            resize_buffers(pw, ph);
        }
        return;
    }

    if (event.type == SDL_EVENT_MOUSE_WHEEL) {
        if(event.wheel.direction==SDL_MOUSEWHEEL_FLIPPED)event.wheel.y=-event.wheel.y;
        if(m_mode==Sidebar::Mode::Settings || (m_mode==Sidebar::Mode::Library && m_experience.navigation.current.page!=LibraryPage::Grid)){m_experience.scroll(event.wheel.y);return;}
        if (m_mode == Sidebar::Mode::Library && m_album_grid) {
            m_album_grid->handle_wheel(event.wheel.y);
            m_needs_redraw = true;
        }
        return;
    }

    if (event.type == SDL_EVENT_TEXT_INPUT) {
        if(m_mode==Sidebar::Mode::Settings){m_experience.text(event.text.text);return;}
        if (m_mode == Sidebar::Mode::Library && m_album_grid && m_album_grid->search_active) {
            if (m_album_grid->handle_text_input(event.text.text)) {
                m_needs_redraw = true;
                return;
            }
        }
    }

    if (event.type == SDL_EVENT_KEY_DOWN) {
        if(m_scope_selector->handle_key_down(event.key.key))return;
        if(m_mode==Sidebar::Mode::Settings && m_experience.key(event.key.key,event.key.mod))return;
        if(event.key.key==SDLK_COMMA && (event.key.mod & SDL_KMOD_GUI)){set_mode(Sidebar::Mode::Settings);return;}
        if(event.key.key==SDLK_ESCAPE && m_scope_selector->menu_open){m_scope_selector->close_menu();return;}
        if(event.key.key==SDLK_ESCAPE && m_experience.menu_open){m_experience.menu_open=false;return;}
        if(event.key.key==SDLK_ESCAPE && m_mode==Sidebar::Mode::Settings){set_mode(Sidebar::Mode::Library);return;}
        if(event.key.key==SDLK_ESCAPE && m_mode==Sidebar::Mode::Library && m_experience.navigation.current.page!=LibraryPage::Grid){m_experience.back();update_back_label();return;}
        if (m_mode == Sidebar::Mode::Library && m_album_grid && m_album_grid->search_active) {
            if (m_album_grid->handle_key_down(event.key.key)) {
                m_needs_redraw = true;
                return;
            }
        }

        if (event.key.key == SDLK_ESCAPE) {
            if(!m_album_grid->genre_filter.empty()){m_album_grid->set_category(2);return;}
            if (m_modal_alpha.val > 0.05f) {
                show_sparklist_modal(false);
            } else {
                m_running = false;
            }
        } else if (event.key.key == SDLK_D) {
            set_mode(m_mode == Sidebar::Mode::Library ? Sidebar::Mode::Devices : Sidebar::Mode::Library);
        } else if (event.key.key == SDLK_S) {
            show_sparklist_modal(!m_show_modal);
        } else if (event.key.key == SDLK_1) {
            set_window_size_points(720, 480);
        } else if (event.key.key == SDLK_2) {
            set_window_size_points(960, 640);
        } else if (event.key.key == SDLK_3) {
            set_window_size_points(1200, 800);
        } else if (event.key.key == SDLK_4) {
            set_window_size_points(1440, 900);
        } else if (event.key.key == SDLK_M || event.key.key == SDLK_F) {
            toggle_maximize();
        } else if (event.key.key == SDLK_EQUALS || event.key.key == SDLK_PLUS) {
            int nw = std::min(2560, m_logical_w + 120);
            int nh = std::min(1600, (int)(nw * 480.0f / 720.0f + 0.5f));
            set_window_size_points(nw, nh);
        } else if (event.key.key == SDLK_MINUS) {
            int nw = std::max(720, m_logical_w - 120);
            int nh = std::max(480, (int)(nw * 480.0f / 720.0f + 0.5f));
            set_window_size_points(nw, nh);
        }
        return;
    }

    if (event.type == SDL_EVENT_MOUSE_BUTTON_DOWN) {
        float x = event.button.x;
        float y = event.button.y;
        int btn = event.button.button;

        if (m_modal_alpha.val > 0.05f) {
            if (m_modal->handle_mouse_down(x, y, btn)) {
                m_modal_dirty = true;
                return;
            }
            // Click outside closes modal with spring animation
            show_sparklist_modal(false);
            return;
        }

        if(m_scope_selector->menu_open && m_scope_selector->handle_mouse_down(x,y,btn))return;
        if(m_experience.menu_open && m_experience.click(x,y))return;
        if (m_traffic_lights->handle_mouse_down(x, y, btn)) return;
        if (m_scope_selector->handle_mouse_down(x, y, btn)) return;
        if(m_mode==Sidebar::Mode::Settings){
            if(y>=64)m_experience.click(x,y);
            else if(m_now_playing->handle_mouse_down(x,y,btn))return;
            return;
        }
        if (m_now_playing->handle_mouse_down(x, y, btn)) return;
        if (m_sidebar->handle_mouse_down(x, y, btn)) return;

        if (m_mode == Sidebar::Mode::Library) {
            if(m_experience.navigation.current.page!=LibraryPage::Grid){m_experience.click(x,y);update_back_label();return;}
            if (m_album_grid->handle_mouse_down(x, y, btn)) return;
        } else {
            if (m_puffy_toggle->handle_mouse_down(x, y, btn)) return;
            if (m_manual_sync_card->handle_mouse_down(x, y, btn)) return;
            if (m_qualitizer_card->handle_mouse_down(x, y, btn)) return;
        }
    } else if (event.type == SDL_EVENT_MOUSE_BUTTON_UP) {
        m_experience.pointer_up();
        float x = event.button.x;
        float y = event.button.y;
        int btn = event.button.button;

        if (m_modal_alpha.val > 0.05f) {
            m_modal->handle_mouse_up(x, y, btn);
            return;
        }
        m_traffic_lights->handle_mouse_up(x, y, btn);
        m_scope_selector->handle_mouse_up(x, y, btn);
        m_now_playing->handle_mouse_up(x, y, btn);
        m_sidebar->handle_mouse_up(x, y, btn);
        if (m_mode == Sidebar::Mode::Library) {
            m_album_grid->handle_mouse_up(x, y, btn);
        } else if(m_mode==Sidebar::Mode::Devices) {
            m_puffy_toggle->handle_mouse_up(x, y, btn);
            m_manual_sync_card->handle_mouse_up(x, y, btn);
            m_qualitizer_card->handle_mouse_up(x, y, btn);
        }
    } else if (event.type == SDL_EVENT_MOUSE_MOTION) {
        float x = event.motion.x;
        float y = event.motion.y;
        m_experience.pointer_move(x,y);

        if (m_cursor_pointer && m_cursor_default) {
            int edges=resize_edges(raw_event.motion.x,raw_event.motion.y);
            if(edges){
                int cursor=((edges&3) && (edges&12))?(((edges&1) && (edges&4)) || ((edges&2) && (edges&8))?2:3):((edges&3)?0:1);
                SDL_SetCursor(m_resize_cursors[cursor]);
            } else if (is_interactive_point(x, y)) {
                SDL_SetCursor(m_cursor_pointer);
            } else {
                SDL_SetCursor(m_cursor_default);
            }
        }

        if (m_modal_alpha.val > 0.05f) {
            m_modal->handle_mouse_move(x, y);
            return;
        }
        m_traffic_lights->handle_mouse_move(x, y);
        m_scope_selector->handle_mouse_move(x, y);
        m_now_playing->handle_mouse_move(x, y);
        m_sidebar->handle_mouse_move(x, y);
        if (m_mode == Sidebar::Mode::Library) {
            m_album_grid->handle_mouse_move(x, y);
        } else if(m_mode==Sidebar::Mode::Devices) {
            m_puffy_toggle->handle_mouse_move(x, y);
            m_manual_sync_card->handle_mouse_move(x, y);
            m_qualitizer_card->handle_mouse_move(x, y);
        }
    }
}

bool Application::has_active_animations() const {
    if(m_experience.animating())return true;
    if (m_modal_scale.is_animating() || m_modal_alpha.is_animating() || m_view_transition.is_animating()) return true;
    if (m_modal && m_modal->is_animating()) return true;
    if (m_scope_selector && m_scope_selector->is_animating()) return true;
    if (m_now_playing && m_now_playing->is_animating()) return true;
    if (m_sidebar && m_sidebar->is_animating()) return true;
    if (m_mode == Sidebar::Mode::Library) {
        if (m_album_grid && m_album_grid->is_animating()) return true;
    } else {
        if (m_puffy_toggle && m_puffy_toggle->is_animating()) return true;
        if (m_manual_sync_card && m_manual_sync_card->tab_bar && m_manual_sync_card->tab_bar->is_animating()) return true;
        if (m_qualitizer_card && m_qualitizer_card->is_animating()) return true;
    }
    return false;
}

void Application::run() {
    if (!m_dump_png.empty()) {
        save_screenshot(m_dump_png);
        return;
    }

    const uint64_t freq = SDL_GetPerformanceFrequency();
    const double target_frame_time = 1.0 / 120.0; // 120 fps = ~8.333ms per frame

    uint64_t frame_start = SDL_GetPerformanceCounter();
    uint64_t last_time = frame_start;
    uint64_t last_fps_report = frame_start;
    int frame_count = 0;

    SDL_Event event;
    while (m_running) {
        bool active = has_active_animations() || m_needs_redraw;

        if (!active) {
            // When idle: wait for events with timeout so zero CPU is wasted
            if (SDL_WaitEventTimeout(&event, 50)) {
                handle_event(event);
                while (SDL_PollEvent(&event)) {
                    handle_event(event);
                }
            } else {
                continue;
            }
            last_time = SDL_GetPerformanceCounter();
        } else {
            // When animating/playing: poll any pending events immediately
            while (SDL_PollEvent(&event)) {
                handle_event(event);
            }
        }

        uint64_t now = SDL_GetPerformanceCounter();
        double dt = (double)(now - last_time) / (double)freq;
        last_time = now;
        if (dt < 0.001) dt = 0.001;
        if (dt > 0.05) dt = 0.05;

        frame_start = now;

        if (m_now_playing && m_now_playing->state.is_playing && !m_now_playing->is_scrubbing) {
            auto track=m_experience.library.tracks.find(m_experience.sequence.current());
            float seconds=track==m_experience.library.tracks.end()?180.f:float(track->second.seconds);
            m_now_playing->state.progress += (float)(dt / seconds);
            if (m_now_playing->state.progress > 1.0f) {
                if(m_now_playing->state.repeat_on)m_now_playing->state.progress=0;
                else m_experience.advance(1);
            }
            m_needs_redraw = true;
        }

        update_animations((float)dt);

        render_frame();
        m_needs_redraw = false;

        frame_count++;

        now = SDL_GetPerformanceCounter();
        if ((double)(now - last_fps_report) / (double)freq >= 1.0) {
            double fps = (double)frame_count / ((double)(now - last_fps_report) / (double)freq);
            std::cout << "[MusicOS] Performance: " << (int)(fps + 0.5) << " FPS\n" << std::flush;
            frame_count = 0;
            last_fps_report = now;
        }

        // Precision 120 FPS pacing using high-precision sleep without busy-spinning
        now = SDL_GetPerformanceCounter();
        double elapsed = (double)(now - frame_start) / (double)freq;
        if (elapsed < target_frame_time) {
            uint64_t remaining_ns = (uint64_t)((target_frame_time - elapsed) * 1e9);
            SDL_DelayPrecise(remaining_ns);
        }
    }
}


SDL_HitTestResult SDLCALL Application::hit_test_cb(SDL_Window* /*win*/, const SDL_Point* area, void* data) {
    Application* app = static_cast<Application*>(data);
    return app->hit_test(area->x, area->y);
}

void Application::update_animations(float dt) {
    m_experience.update(dt,m_now_playing->state.cover_path,m_mode==Sidebar::Mode::Library);
    if (m_scope_selector) m_scope_selector->update_animations(dt);
    if (m_now_playing) m_now_playing->update_animations(dt);
    if (m_album_grid) m_album_grid->update_animations(dt);
    if (m_sidebar) m_sidebar->update_animations(dt);
    if (m_puffy_toggle) m_puffy_toggle->update_animations(dt);
    if (m_manual_sync_card && m_manual_sync_card->tab_bar) m_manual_sync_card->tab_bar->update_animations(dt);
    if (m_qualitizer_card) m_qualitizer_card->update_animations(dt);
    if (m_modal) m_modal->update_animations(dt);

    if (m_sidebar && m_sidebar->is_animating()) m_dev_dirty = true;
    if (m_modal && m_modal->is_animating()) m_modal_dirty = true;

    m_modal_scale.update(dt);
    m_modal_alpha.update(dt);
    m_view_transition.update(dt);

    if (!m_show_modal && m_modal_alpha.val < 0.005f) {
        m_modal_bg_cached = false;
    }
}

SDL_HitTestResult Application::hit_test(int x, int y) {
    if (m_modal_alpha.val > 0.05f) {
        return SDL_HITTEST_NORMAL;
    }

    int win_w = m_logical_w, win_h = m_logical_h;
    if (m_window) {
        SDL_GetWindowSize(m_window, &win_w, &win_h);
    }

    // Resize gestures are handled in window coordinates for every platform.
    if(resize_edges(float(x),float(y)))return SDL_HITTEST_NORMAL;

    float lx = win_w > 0 ? (float)x * (float)m_logical_w / (float)win_w : (float)x;
    float ly = win_h > 0 ? (float)y * (float)m_logical_h / (float)win_h : (float)y;

    if (ly < 62.0f) {
        if (m_traffic_lights && m_traffic_lights->bounds.contains(lx, ly)) {
            return SDL_HITTEST_NORMAL;
        }
        if (m_scope_selector && m_scope_selector->bounds.contains(lx, ly)) {
            return SDL_HITTEST_NORMAL;
        }
        if (m_now_playing && m_now_playing->bounds.contains(lx, ly)) {
            return SDL_HITTEST_NORMAL;
        }
        return SDL_HITTEST_DRAGGABLE;
    }

    return SDL_HITTEST_NORMAL;
}

int Application::resize_edges(float x,float y) const {
    int w=0,h=0;SDL_GetWindowSize(m_window,&w,&h);
    int edges=0;float margin=8;
    if(x<margin)edges|=1;else if(x>w-margin)edges|=2;
    if(y<margin)edges|=4;else if(y>h-margin)edges|=8;
    return edges;
}

bool Application::is_interactive_point(float x, float y) const {
    if(m_mode==Sidebar::Mode::Settings)return y>=64;
    if(m_scope_selector->menu_open && m_scope_selector->menu_bounds().contains(x,y))return true;
    if (m_modal_alpha.val > 0.05f) {
        if (m_modal && m_modal->bounds.contains(x, y)) return true;
        return false;
    }

    if (m_traffic_lights && m_traffic_lights->bounds.contains(x, y)) return true;
    if (m_scope_selector && m_scope_selector->bounds.contains(x, y)) return true;
    if (m_now_playing && m_now_playing->bounds.contains(x, y)) return true;

    if (m_mode == Sidebar::Mode::Library) {
        if (m_sidebar && m_sidebar->bounds.contains(x, y)) return true;
        if (m_album_grid && m_album_grid->bounds.contains(x, y)) return true;
    } else {
        if (m_sidebar && m_sidebar->bounds.contains(x, y)) return true;
        if (m_puffy_toggle && m_puffy_toggle->bounds.contains(x, y)) return true;
        if (m_manual_sync_card && m_manual_sync_card->bounds.contains(x, y)) return true;
        if (m_qualitizer_card && m_qualitizer_card->bounds.contains(x, y)) return true;
    }

    return false;
}

} // namespace MusicUI
