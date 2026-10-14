#include "uilib/Application.hpp"
#include "uilib/AudioEngine.hpp"
#include "uilib/MusicSearchService.hpp"
#include <chrono>
#include <cassert>
#include <iostream>

namespace MusicUI {
struct DesktopChecks {
    static void run() {
        std::filesystem::create_directories("out/library-review/checks");
        const std::string prefs="out/library-review/checks/preferences.txt";
        assert(Preferences::load("out/library-review/checks/absent.txt").display_name=="Henry");
        Preferences p;p.display_name="  Zoë \"Music\"  ";p.coverflow_colors=true;p.interface_percent=125;
        assert(p.save(prefs));auto loaded=Preferences::load(prefs);
        assert(loaded.display_name=="Zoë \"Music\"");assert(loaded.coverflow_colors);assert(loaded.interface_percent==125);
        std::ofstream(prefs)<<"name \" \"\nscale 999\ncolors 9\n";
        loaded=Preferences::load(prefs);assert(loaded.display_name=="Henry");assert(loaded.interface_percent==100);assert(!loaded.coverflow_colors);
        assert(!p.save("out/library-review/checks/missing-directory/preferences.txt"));
        std::string unicode="Zoë";erase_character(unicode);assert(unicode=="Zo");
        assert(!Preferences::valid_name(std::string(49,'a')));
        SpringAnim stalled(0,650,51);stalled.set_target(1);stalled.update(1);assert(std::isfinite(stalled.val));assert(std::abs(stalled.val-1)<.001f);
        SpringAnim slow(0,650,51),fast(0,650,51);slow.set_target(1);fast.set_target(1);
        for(int i=0;i<18;++i)slow.update(1.f/30);for(int i=0;i<72;++i)fast.update(1.f/120);assert(std::abs(slow.val-fast.val)<.0001f);
        TimedAnim repeated(0,.14f);repeated.set_target(1);for(int i=0;i<20;++i){repeated.set_target(1);repeated.update(.016f);}assert(!repeated.is_animating() && repeated.val==1);
        DemoLibrary library;auto& album=library.albums.front();
        DemoSequence sequence;sequence.start(album.tracks,false,album.tracks[3]);assert(sequence.current()==album.tracks[3]);sequence.next(1);assert(sequence.current()==album.tracks[4]);sequence.next(-1);assert(sequence.current()==album.tracks[3]);
        sequence.start(album.tracks,true);auto sorted=sequence.ids;std::sort(sorted.begin(),sorted.end());auto expected=album.tracks;std::sort(expected.begin(),expected.end());assert(sorted==expected);assert(sequence.shuffled);auto selected=sequence.current();sequence.set_shuffle(false);assert(sequence.ids==album.tracks);assert(sequence.current()==selected);
        LibraryNavigation nav;nav.current.scroll=120;nav.open(LibraryPage::Album,album.id);nav.current.scroll=50;nav.open(LibraryPage::Artist,album.artist_id);assert(nav.back());assert(nav.current.scroll==50);assert(nav.back());assert(nav.current.scroll==120);
        assert(theme_artwork(library,{LibraryPage::Album,album.id,0},"playing.png")==album.cover);
        assert(theme_artwork(library,{LibraryPage::Artist,"m83",0},"playing.png")=="assets/artists/m83.png");
        assert(theme_artwork(library,{LibraryPage::Artist,album.artist_id,0},"playing.png")==album.cover);
        assert(theme_artwork(library,{},"playing.png")=="playing.png");

        Application app;
        const char* args[]={"musicos-checks","--points","1280x850","--dump-png","out/library-review/checks/unused.png"};
        assert(app.init(5,const_cast<char**>(args)));
        app.m_experience.preferences_path=prefs;
        auto click=[&](float x,float y){
            SDL_Event e{};e.type=SDL_EVENT_MOUSE_BUTTON_DOWN;e.button.button=SDL_BUTTON_LEFT;
            int ww=0,wh=0;SDL_GetWindowSize(app.m_window,&ww,&wh);
            e.button.x=x*ww/app.m_logical_w;e.button.y=y*wh/app.m_logical_h;
            app.handle_event(e);app.update_animations(.016f);app.m_experience.update(1,app.m_now_playing->state.cover_path);app.render_frame();
            assert(cairo_status(app.m_canvas->cr())==CAIRO_STATUS_SUCCESS);
        };
        std::string requested_genre;
        app.m_album_grid->on_search_catalog=[&](const std::string&,int,const std::string& genre){requested_genre=genre;};
        app.render_frame();
        assert(SDL_GetWindowFlags(app.m_window)&SDL_WINDOW_TRANSPARENT);
        assert(app.resize_edges(2,400)==1);assert(app.resize_edges(1278,848)==10);
        int closed=0,minimized=0,maximized=0;
        auto close=app.m_traffic_lights->on_close,mini=app.m_traffic_lights->on_minimize,maxi=app.m_traffic_lights->on_maximize;
        app.m_traffic_lights->on_close=[&]{++closed;};app.m_traffic_lights->on_minimize=[&]{++minimized;};app.m_traffic_lights->on_maximize=[&]{++maximized;};
        click(30,31);click(66,31);click(102,31);assert(closed==1 && minimized==1 && maximized==1);
        app.m_traffic_lights->on_close=close;app.m_traffic_lights->on_minimize=mini;app.m_traffic_lights->on_maximize=maxi;
        click(100,134);assert(app.m_album_grid->category==1);click(280,170);assert(app.m_experience.navigation.current.page==LibraryPage::Artist);assert(app.m_experience.navigation.current.id==album.artist_id);
        app.m_experience.back();click(100,168);assert(app.m_album_grid->category==2);click(280,170);assert(app.m_album_grid->genre_filter=="Alternative" && requested_genre=="20");
        click(100,202);assert(app.m_album_grid->category==3);assert(app.m_album_grid->cards.size()>64);
        app.render_frame();app.save_screenshot("out/library-repair/songs-large.png");click(290,180);assert(!app.m_experience.sequence.current().empty());
        app.m_now_playing->state.is_playing=false;AudioEngine::instance().pause();
        app.m_experience.navigation.grid();app.m_sidebar->nav_selected=0;app.m_album_grid->set_category(0);app.render_frame();
        click(240,30);assert(app.m_scope_selector->menu_open);
        SDL_Event menu_key{};menu_key.type=SDL_EVENT_KEY_DOWN;menu_key.key.key=SDLK_DOWN;app.handle_event(menu_key);
        menu_key.key.key=SDLK_RETURN;app.handle_event(menu_key);assert(app.m_mode==Sidebar::Mode::Devices);assert(!app.m_scope_selector->menu_open);
        app.set_mode(Sidebar::Mode::Library);app.m_scope_selector->menu_reveal.snap(0);app.render_frame();click(280,170);
        assert(app.m_experience.navigation.current.id==album.id);
        assert(!app.m_now_playing->state.is_playing);
        assert(app.m_now_playing->state.cover_path.empty());
        click(450,155);
        assert(app.m_experience.navigation.current.page==LibraryPage::Artist);
        click(100,100);
        assert(app.m_experience.navigation.current.page==LibraryPage::Album);
        click(245,310);assert(app.m_experience.sequence.current()==album.tracks.front());assert(app.m_now_playing->state.is_playing);
        app.m_now_playing->on_next();assert(app.m_experience.sequence.current()==album.tracks[1]);
        app.m_now_playing->on_prev();assert(app.m_experience.sequence.current()==album.tracks.front());
        // Header overflow actions and artist sequence use the same records.
        
        click(501,229);assert(app.m_experience.menu_open);
        click(430,303);assert(app.m_experience.sequence.shuffled);assert(app.m_now_playing->state.shuffle_on);
        click(501,229);click(430,337);
        assert(app.m_experience.navigation.current.page==LibraryPage::Artist);
        app.m_experience.start();assert(app.m_experience.sequence.ids==library.artist_tracks(album.artist_id));
        app.m_experience.back();app.render_frame();
        app.m_experience.scroll(-3);app.render_frame();assert(app.m_experience.navigation.current.scroll>0);
        auto detail_scroll=app.m_experience.navigation.current.scroll;
        app.m_experience.open(LibraryPage::Artist,album.artist_id);assert(app.m_experience.back());assert(app.m_experience.navigation.current.scroll==detail_scroll);
        app.m_album_grid->scroll_y=80;app.m_album_grid->target_scroll_y=80;app.m_album_grid->selected_index=2;
        assert(app.m_experience.back());assert(app.m_experience.navigation.current.page==LibraryPage::Grid);
        assert(app.m_album_grid->scroll_y==80 && app.m_album_grid->selected_index==2);
        app.m_experience.open(LibraryPage::Album,album.id);app.render_frame();
        app.m_now_playing->state.is_playing=false;AudioEngine::instance().pause();
        app.m_experience.navigation.current.scroll=90;
        app.set_mode(Sidebar::Mode::Settings);app.render_frame();
        app.m_experience.editing_name=true;app.m_experience.name_draft="  Leaving Settings  ";
        click(240,30);assert(app.m_scope_selector->menu_open);
        click(200,86);assert(app.m_mode==Sidebar::Mode::Library);assert(app.m_experience.navigation.current.scroll==90);assert(app.m_sidebar->display_name=="Leaving Settings");
        app.set_mode(Sidebar::Mode::Settings);app.render_frame();
        click(1044,198);assert(app.m_experience.preferences.coverflow_colors);
        click(918,472);assert(app.m_experience.preferences.interface_percent==125);
        app.m_experience.preferences.interface_percent=100;app.preferences_changed();app.render_frame();
        app.m_experience.name_draft="  Test Listener  ";assert(app.m_experience.commit_name());assert(app.m_sidebar->display_name=="Test Listener");
        auto before=app.m_experience.preferences.display_name;app.m_experience.name_draft="   ";assert(!app.m_experience.commit_name());assert(app.m_experience.preferences.display_name==before);app.m_experience.name_draft=before;
        for(int percent:{80,100,125,150}) {
            app.m_experience.preferences.interface_percent=percent;app.preferences_changed();app.set_mode(Sidebar::Mode::Library);
            app.m_experience.navigation.grid();app.m_album_grid->set_category(0);app.render_frame();
            click(280,170);assert(app.m_experience.navigation.current.id==album.id);
            int ww=0,wh=0;SDL_GetWindowSize(app.m_window,&ww,&wh);float wx=240.f*ww/app.m_logical_w,wy=30.f*wh/app.m_logical_h;
            assert(app.hit_test(int(wx),int(wy))==SDL_HITTEST_NORMAL);
            app.save_screenshot("out/library-review/checks/album-scale-"+std::to_string(percent)+".png");
            app.set_mode(Sidebar::Mode::Settings);app.render_frame();
            app.save_screenshot("out/library-review/checks/settings-scale-"+std::to_string(percent)+".png");
            app.set_mode(Sidebar::Mode::Library);
        }
        app.m_experience.preferences.coverflow_colors=false;app.m_experience.open(LibraryPage::Artist,"m83");app.m_experience.update(1,"");
        auto blue=Color::from_hex(0x00AEFF);auto mapped=app.m_experience.remap(blue);assert(std::abs(mapped.b-blue.b)<.001f && std::abs(mapped.g-blue.g)<.001f);
        app.m_experience.preferences.coverflow_colors=true;
        auto& first=app.m_experience.library.albums.front();auto original=first.cover;first.cover="out/library-review/missing.png";
        app.m_experience.navigation.current={LibraryPage::Album,first.id,0};app.m_experience.update(1,"");mapped=app.m_experience.remap(blue);assert(std::abs(mapped.g-blue.g)<.001f);
        app.render_frame();app.save_screenshot("out/library-review/checks/missing-artwork.png");
        first.cover=original;
        app.m_experience.update(1,"",false);mapped=app.m_experience.remap(blue);assert(std::abs(mapped.g-blue.g)<.001f);
        app.m_experience.navigation.current={LibraryPage::Artist,"m83",0};app.m_experience.update(1,"");mapped=app.m_experience.remap(blue);assert(std::abs(mapped.g-blue.g)>.01f);
        auto luminance=[](Color color){auto linear=[](float v){return v<=.04045f?v/12.92f:std::pow((v+.055f)/1.055f,2.4f);};return .2126f*linear(color.r)+.7152f*linear(color.g)+.0722f*linear(color.b);};
        for(auto& artist:app.m_experience.library.artists)if(artist.id=="m83"){
            auto portrait=artist.portrait;artist.portrait="out/library-review/missing-portrait.png";
            app.m_experience.update(1,"");auto fallback=app.m_experience.remap(blue);
            app.m_experience.navigation.current={LibraryPage::Album,artist.albums.front(),0};app.m_experience.update(1,"");auto expected=app.m_experience.remap(blue);
            assert(std::abs(fallback.g-expected.g)<.001f);artist.portrait=portrait;
        }
        app.m_experience.navigation.current={};
        for(const auto& cover:{"assets/covers/RAM.png","assets/covers/Clouds.png"}){
            app.m_experience.update(1,cover);
            Color surface=app.m_experience.remap(Color::from_hex(0xD6EDFD));
            Color accent=app.m_experience.remap(Color::from_hex(0x0064DF));
            assert((luminance(surface)+.05f)/.05f>=4.5f);
            assert(1.05f/(luminance(accent)+.05f)>=4.5f);
            app.m_now_playing->state.cover_path=cover;app.render_frame();
            app.save_screenshot(std::string("out/library-review/checks/")+(std::string(cover).find("RAM")!=std::string::npos?"dark-artwork.png":"light-artwork.png"));
        }
        auto records=MusicSearchService::parse(R"({"results":[{"wrapperType":"artist","artistId":987,"artistName":"Test Artist"},{"wrapperType":"collection","artistId":987,"artistName":"Test Artist","collectionId":654,"collectionName":"Test Album","primaryGenreName":"Jazz"},{"wrapperType":"track","artistId":987,"artistName":"Test Artist","collectionId":654,"collectionName":"Test Album","trackId":321,"trackName":"Song \"quoted\"","trackTimeMillis":203000,"primaryGenreName":"Jazz"}]})");
        assert(records.size()==3);app.m_experience.library.import_catalog(records);app.m_experience.library.import_catalog(records);
        auto catalog_album=app.m_experience.library.album("apple/album/654");assert(catalog_album && catalog_album->tracks.size()==1 && catalog_album->genre=="Jazz");
        assert(app.m_experience.library.tracks.at("apple/track/321").seconds==203);
        assert(app.m_experience.library.artist("apple/artist/987")->albums.size()==1);
        std::string lookup;app.m_experience.on_catalog_lookup=[&](const std::string& id,bool){lookup=id;};
        app.m_experience.open(LibraryPage::Artist,"apple/artist/987");assert(lookup=="apple/artist/987");
        app.m_experience.open(LibraryPage::Album,"apple/album/654");assert(lookup=="apple/album/654");
        app.m_experience.start();assert(app.m_experience.sequence.current()=="apple/track/321");
        app.m_now_playing->state.is_playing=false;AudioEngine::instance().pause();
        assert(MusicSearchService::parse("invalid").empty());
        app.m_experience.preferences.interface_percent=100;app.m_experience.preferences.display_name="Henry";app.m_experience.preferences.coverflow_colors=false;app.m_experience.save_preferences();
        auto restarted=Preferences::load(prefs);assert(restarted.display_name=="Henry" && restarted.interface_percent==100 && !restarted.coverflow_colors);
        app.m_experience.navigation.grid();app.m_sidebar->nav_selected=3;app.m_album_grid->set_category(3);app.m_experience.preferences.coverflow_colors=false;app.preferences_changed();
        SDL_SetRenderVSync(app.m_renderer,0);app.render_frame();
        std::vector<double> costs;for(int i=0;i<30;++i){auto begin=std::chrono::steady_clock::now();app.render_frame();costs.push_back(std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-begin).count());}
        std::sort(costs.begin(),costs.end());std::cout<<"Warm Songs render median: "<<costs[costs.size()/2]<<" ms at 2x density.\n";
        cairo_surface_flush(app.m_cairo_surface);auto pixel=reinterpret_cast<uint32_t*>(cairo_image_surface_get_data(app.m_cairo_surface));assert((pixel[0]>>24)==0);
        std::cout<<"Desktop checks passed: preferences, routes, sequences, theme precedence/fallback, settings selector, and all four scaled hit targets.\n";
    }
};
}
int main(){MusicUI::DesktopChecks::run();}
