#include "LibraryExperience.hpp"
#include <SDL3_image/SDL_image.h>
#include <array>
#include <cmath>

namespace MusicUI {
namespace {
Color mix(Color a, Color b, float t) { return {a.r+(b.r-a.r)*t,a.g+(b.g-a.g)*t,a.b+(b.b-a.b)*t,a.a+(b.a-a.a)*t}; }
void fitted(Canvas& c, std::string text, Point at, float size, float width, FontWeight weight = FontWeight::Medium, TextAlign align = TextAlign::Left) {
    if (width <= 0) return;
    if (c.measure_text(text, size, weight).w > width) {
        while (!text.empty() && c.measure_text(text + "…", size, weight).w > width) erase_character(text);
        text += "…";
    }
    c.draw_text(text, at, size, Color::Black, weight, align);
}
void wrapped(Canvas& c, const std::string& text, Rect r, float size = 12) {
    std::istringstream stream(text);std::string word,line;std::vector<std::string> lines;
    while(stream>>word){
        auto next=line.empty()?word:line+" "+word;
        if(!line.empty() && c.measure_text(next,size,FontWeight::Light).w>r.w){lines.push_back(line);line=word;}
        else line=next;
    }
    if(!line.empty())lines.push_back(line);
    int count=std::max(1,int(r.h/(size+2)));
    if(int(lines.size())>count){lines.resize(count);lines.back()+="…";}
    for(size_t i=0;i<lines.size();++i)fitted(c,lines[i],{r.x,r.y+float(i)*(size+2)},size,r.w,FontWeight::Light);

}
}
LibraryExperience::LibraryExperience() {
    char* root = SDL_GetPrefPath("MusicOS", "MusicOSDesktop");
    if (root) { preferences_path = std::string(root) + "preferences.txt"; SDL_free(root); }
    preferences = Preferences::load(preferences_path);
    name_draft = preferences.display_name;
}
LibraryExperience::~LibraryExperience() { for (auto& p : backgrounds) cairo_surface_destroy(p.second); }
void LibraryExperience::save_preferences() {
    save_error = preferences.save(preferences_path) ? "" : "Could not save settings. Your changes are active for this session.";
    if (on_preferences_changed) on_preferences_changed();
}
bool LibraryExperience::commit_name() {
    auto name = trimmed(name_draft);
    if (!Preferences::valid_name(name)) { save_error = "Enter a name (up to 48 characters)."; return false; }
    // Limit by Unicode code points while preserving multibyte names.
    size_t characters = 0; for (unsigned char c : name) if ((c & 0xc0) != 0x80) ++characters;
    if (characters > 48) { save_error = "Use 48 characters or fewer."; return false; }
    preferences.display_name=name; name_draft=name; save_preferences(); return true;
}
void LibraryExperience::open(LibraryPage page, const std::string& id) {
    if ((page==LibraryPage::Album && !library.album(id)) || (page==LibraryPage::Artist && !library.artist(id))) return;
    navigation.open(page,id); data_status.clear();
    if(page==LibraryPage::Album){auto a=library.album(id);if(a && !a->catalog_id.empty() && on_catalog_lookup){data_status="Loading album tracks…";on_catalog_lookup(id,false);}}
    if(page==LibraryPage::Artist){auto a=library.artist(id);if(a && !a->catalog_id.empty() && on_catalog_lookup){data_status="Loading artist albums…";on_catalog_lookup("apple/artist/"+a->catalog_id,true);}}
    menu_open=false; editing_name=false; reveal();scroll_position.snap(0);
}
bool LibraryExperience::back() { menu_open=false;bool result=navigation.back();if(result){reveal();scroll_position.snap(navigation.current.scroll);}return result; }
void LibraryExperience::start(bool shuffle, const std::string& track) {
    auto route=navigation.current; std::vector<std::string> list;
    if (route.page==LibraryPage::Album) { if (auto a=library.album(route.id)) list=a->tracks; }
    if (route.page==LibraryPage::Artist) list=library.artist_tracks(route.id);
    if (list.empty()) return;
    sequence.start(std::move(list),shuffle,track);
    if (on_playback_changed) on_playback_changed();
}
void LibraryExperience::advance(int delta) { if (sequence.ids.empty()) return; sequence.next(delta); if(on_playback_changed) on_playback_changed(); }
void LibraryExperience::add_hit(Rect r, std::function<void()> action) { hits.push_back({r,std::move(action)}); }
void LibraryExperience::icon_button(Canvas& c, Rect r, const std::string& icon, std::function<void()> action, bool pill) {
    if(pill || r.contains(pointer.x,pointer.y)) c.fill_rounded_rect(r,10,Color::from_hex(r.contains(pointer.x,pointer.y)?0xAAE4FF:0xD6EDFD));
    c.draw_svg("assets/icons/sketch/"+icon+".svg",r);
    add_hit(r,std::move(action));
}
void LibraryExperience::overflow(Canvas& c, Rect r, const std::string& track) {
    icon_button(c,r,"3dots_(nocircle)",[this,r,track]{menu_open=!menu_open; menu_track=track; menu_rect={r.right()-170,r.bottom()+4,170,112};},true);
}
void LibraryExperience::render(Canvas& c, Rect b) {
    hits.clear(); menu_hits.clear(); rendered_bounds=b;scroll_rail={};
    c.fill_rounded_rect(b,16,Color::from_hex(settings?0xCBF1FF:0x00AEFF));
    if(preferences.coverflow_colors) render_background(c,b);
    c.stroke_rounded_rect(b,16,Color::from_hex(0x0018FF),1);
    c.push_clip_rounded_rect(b,16);
    auto cr=c.cr();bool entering=page_reveal.is_animating();
    if(entering){cairo_save(cr);cairo_push_group(cr);cairo_translate(cr,0,5*(1-page_reveal.val));}
    if(settings) render_settings(c,b);
    else if(navigation.current.page==LibraryPage::Album) render_album(c,b);
    else if(navigation.current.page==LibraryPage::Artist) render_artist(c,b);
    if(entering){cairo_pop_group_to_source(cr);cairo_paint_with_alpha(cr,.82f+.18f*page_reveal.val);cairo_restore(cr);}
    if(content_height>b.h+.5f) {
        float travel=std::max(28.f,b.h-80);
        scroll_thumb=std::max(28.f,travel*b.h/content_height);
        scroll_rail={b.right()-11,b.y+40,8,travel};
        float offset=scroll_position.val;
        float top=scroll_rail.y+(travel-scroll_thumb)*offset/(content_height-b.h);
        c.fill_rounded_rect(scroll_rail,4,Color::from_hex(0x0034FF));
        c.fill_rounded_rect({scroll_rail.x+1,top,6,scroll_thumb},3,Color::White);
    }
    c.pop_clip();
    if(menu_open) render_menu(c);
}
void LibraryExperience::render_album(Canvas& c, Rect b) {
    auto a=library.album(navigation.current.id); if(!a) return;
    content_height=176+float(a->tracks.size())*52+20;
    navigation.current.scroll=std::clamp(navigation.current.scroll,0.f,std::max(0.f,content_height-b.h));
    float x=b.x, y=b.y-scroll_position.val;
    bool wide=b.w>=760;
    float cover_size=wide?176.f:128.f;
    float text_x=x+21+cover_size+16;
    float rows_top=wide?222.f:176.f;
    content_height=rows_top+float(a->tracks.size())*52+20;
    Rect cover(x+21,y+21,cover_size,cover_size);
    if(std::filesystem::is_regular_file(a->cover)) c.draw_image(a->cover,cover,16);
    else {c.fill_rounded_rect(cover,16,Color::from_hex(0xD6EDFD));c.draw_svg("assets/icons/sketch/noCOVER.svg",cover);}

    auto title=a->title; float title_size=32;
    if(wide)title_size=40;
    if(c.measure_text(title,title_size,FontWeight::UltraBold).w>b.w-(text_x-x)-24) title_size=24;
    fitted(c,title,{text_x,y+21},title_size,b.w-(text_x-x)-24,FontWeight::UltraBold);
    float controls_x=wide?text_x:b.right()-138;
    float controls_y=y+(wide?149:119);
    fitted(c,"Demo audio",{wide?text_x:b.right()-122,y+(wide?119:93)},12,100,FontWeight::Light);
    icon_button(c,{controls_x,controls_y,32,32},"play",[this]{start();},true);
    icon_button(c,{controls_x+36,controls_y,32,32},"ShufflePlay",[this]{start(true);},true);
    overflow(c,{controls_x+72,controls_y,32,32});
    if(wide && !a->store_url.empty()){
        c.draw_text("View in Apple Music ↗",{controls_x+126,controls_y+10},12,Color::Black,FontWeight::Medium);
        add_hit({controls_x+120,controls_y,170,32},[url=a->store_url]{SDL_OpenURL(url.c_str());});
    }
    float artist_y=y+(wide?79:123);
    float artist_x=wide?text_x:x+165;
    float artist_width=wide?b.w-(text_x-x)-48:std::max(30.f,b.w-350);
    c.fill_circle(artist_x+14,artist_y+12,14,Color::from_hex(0xD6DEFD));
    fitted(c,a->artist,{artist_x+32,artist_y},22,artist_width,FontWeight::Light);
    add_hit({artist_x,artist_y-4,artist_width+32,32},[this,id=a->artist_id]{open(LibraryPage::Artist,id);});
    if(a->tracks.empty())wrapped(c,data_status.empty()?"No tracks available for this album.":data_status,{x+24,y+rows_top+12,b.w-48,48},14);
    for(size_t i=0;i<a->tracks.size();++i) {
        Rect row(x+21,y+rows_top+float(i)*52,b.w-48,48);
        if(row.bottom()<b.y || row.y>b.bottom()) continue;
        bool playing=sequence.current()==a->tracks[i];
        c.fill_rounded_rect(row,24,Color::from_hex(playing?0x0064DF:0x0034FF,playing?.3f:(row.contains(pointer.x,pointer.y)?.32f:.20f)));
        if(i<9) c.draw_text(std::to_string(i+1),{row.x+6,row.y+16},16,Color::Black,FontWeight::Medium);
        else c.draw_text(std::to_string(i+1),{row.x+10,row.y+18},10,Color::Black,FontWeight::Medium,TextAlign::Center);
        icon_button(c,{row.x+18,row.y+7,32,32},"play",[this,id=a->tracks[i]]{start(false,id);});
        fitted(c,library.tracks.at(a->tracks[i]).title,{row.x+54,row.y+16},16,row.w-145);
        c.draw_text(duration_text(library.tracks.at(a->tracks[i]).seconds),{row.right()-87,row.y+18},11,Color::Black,FontWeight::Light);
        overflow(c,{row.right()-38,row.y+8,32,32},a->tracks[i]);
    }

}
void LibraryExperience::render_artist(Canvas& c, Rect b) {
    auto a=library.artist(navigation.current.id);if(!a)return;
    bool wide=b.w>=760;
    float y=b.y-scroll_position.val;
    auto art=a->portrait;
    if((art.empty() || !palette(art).valid) && !a->albums.empty())art=library.album(a->albums.front())->cover;
    float portrait_size=wide?192.f:144.f;
    Rect portrait(b.x+24,y+24,portrait_size,portrait_size);
    if(!art.empty() && palette(art).valid)c.draw_image(art,portrait,wide?48:36);
    else {c.fill_rounded_rect(portrait,36,Color::from_hex(0xD6EDFD));c.draw_svg("assets/icons/sketch/noARTIST.svg",portrait);}
    float tx=portrait.right()+24, available=b.right()-tx-24;
    float heading=wide?52.f:36.f;
    while(heading>24 && c.measure_text(a->name,heading,FontWeight::UltraBold).w>available)heading-=2;
    fitted(c,a->name,{tx,y+28},heading,available,FontWeight::UltraBold);
    auto list=library.artist_tracks(a->id);int seconds=0;for(auto& id:list)seconds+=library.tracks.at(id).seconds;
    std::string stats=std::to_string(a->albums.size())+" albums · "+std::to_string(list.size())+" tracks";
    fitted(c,stats,{tx,y+(wide?97:80)},13,available,FontWeight::Medium);
    fitted(c,std::to_string(seconds/60)+" minutes in your library",{tx,y+(wide?118:101)},12,available,FontWeight::Light);
    float controls_y=y+(wide?166:136);
    icon_button(c,{tx,controls_y,32,32},"play",[this]{start();},true);
    icon_button(c,{tx+40,controls_y,32,32},"ShufflePlay",[this]{start(true);},true);
    overflow(c,{tx+80,controls_y,32,32});
    if(!a->store_url.empty() && available>300){
        c.draw_text("View in Apple Music ↗",{tx+126,controls_y+10},12,Color::Black,FontWeight::Medium);
        add_hit({tx+120,controls_y,170,32},[url=a->store_url]{SDL_OpenURL(url.c_str());});
    }
    float about_y=y+(wide?242:180);
    c.fill_rounded_rect({b.x+24,about_y,b.w-48,wide?94.f:78.f},16,Color::from_hex(0xD6EDFD,.65f));
    c.draw_text("About",{b.x+38,about_y+12},16,Color::Black,FontWeight::DemiBold);
    wrapped(c,a->biography.empty()?"No biography available for this artist.":a->biography,{b.x+38,about_y+38,std::min(700.f,b.w-76),wide?44.f:32.f},12);
    float albums_y=about_y+(wide?118:98);
    c.draw_text("Albums",{b.x+24,albums_y},24,Color::Black,FontWeight::Light);
    float strip_y=albums_y+36;
    if(a->albums.empty())wrapped(c,data_status.empty()?"No albums available for this artist.":data_status,{b.x+24,strip_y,b.w-48,48},14);
    int cols=std::max(3,int((b.w-48)/164));
    float cell=(b.w-48)/cols, cover=std::min(128.f,cell-24);
    float cell_height=cover+48;
    int rows=int((a->albums.size()+cols-1)/cols);
    content_height=strip_y-y+rows*cell_height+24;
    navigation.current.scroll=std::clamp(navigation.current.scroll,0.f,std::max(0.f,content_height-b.h));
    for(size_t i=0;i<a->albums.size();++i){
        auto album=library.album(a->albums[i]);
        float cx=b.x+24+cell*(i%cols),cy=strip_y+float(i/cols)*cell_height;
        Rect card(cx,cy,cell-8,cell_height-8);
        if(card.contains(pointer.x,pointer.y))c.fill_rounded_rect(card,16,Color::from_hex(0xD6EDFD,.45f));
        float center=cx+(cell-8)*.5f;
        c.draw_image(album->cover,{center-cover*.5f,cy+8,cover,cover},16);
        fitted(c,album->title,{center,cy+cover+16},12,cell-20,FontWeight::DemiBold,TextAlign::Center);
        add_hit(card,[this,id=album->id]{open(LibraryPage::Album,id);});
    }
}
void LibraryExperience::render_settings(Canvas& c, Rect b) {
    bool stacked=b.w<640; float pad=24, width=std::min(900.f,b.w-48);
    bool compact=b.h<540;
    float appearance_height=compact?150.f:196.f;
    float tile_height=compact?130.f:154.f;
    float field_y=compact?68.f:76.f;
    float field_height=compact?34.f:40.f;
    float hint_y=compact?112.f:126.f;
    content_height=pad+76+appearance_height+16+tile_height+(stacked?16+tile_height:0)+18;
    if(!save_error.empty())content_height+=48;
    settings_scroll=std::clamp(settings_scroll,0.f,std::max(0.f,content_height-b.h));
    float y=b.y+pad-scroll_position.val, x=b.x+(b.w-width)*.5f;
    c.draw_text("Settings",{x,y},32,Color::Black,FontWeight::UltraBold);
    c.draw_text("Appearance and personal preferences.",{x,y+42},14,Color::Black,FontWeight::Light);
    Rect appearance(x,y+76,width,appearance_height);
    c.fill_rounded_rect(appearance,24,Color::from_hex(0xF2FAFF,.96f));
    fitted(c,"Coverflow Colors",{x+20,appearance.y+18},22,width-90,FontWeight::ExtraBold);
    wrapped(c,"Colors follow the album, artist, or playing track.",{x+20,appearance.y+50,width-100,36},14);
    Rect toggle(appearance.right()-72,appearance.y+20,52,28);
    c.fill_pill(toggle,Color::from_hex(preferences.coverflow_colors?0x0064DF:0x869FFF));
    c.fill_circle(toggle.x+14+24*toggle_position.val,toggle.y+14,10,Color::White);
    add_hit(toggle,[this]{preferences.coverflow_colors=!preferences.coverflow_colors;save_preferences();});
    Rect preview(x+20,appearance.y+(compact?82:90),width-40,compact?48:84);
    c.push_clip_rounded_rect(preview,16);
    c.fill_rounded_rect(preview,16,Color::from_hex(0x00AEFF));
    if(current_palette.valid && preferences.coverflow_colors)render_background(c,preview);
    float art_size=compact?32.f:52.f;
    Rect artwork(preview.x+12,preview.y+(preview.h-art_size)*.5f,art_size,art_size);
    if(!playing_artwork.empty() && palette(playing_artwork).valid)c.draw_image(playing_artwork,artwork,8);
    else c.draw_svg("assets/icons/sketch/noCOVER.svg",artwork);
    auto label=preferences.coverflow_colors?(current_palette.valid?"Artwork colors":"Waiting for artwork"):"Original blue";
    auto hint=preferences.coverflow_colors?"Changes as you browse and listen":"Your current appearance";
    c.draw_text(label,{artwork.right()+12,preview.y+(compact?8:20)},13,Color::Black,FontWeight::DemiBold);
    c.draw_text(hint,{artwork.right()+12,preview.y+(compact?27:44)},compact?9.f:11.f,Color::Black,FontWeight::Light);
    c.pop_clip();
    float card_width=stacked?width:(width-16)/2;
    Rect name(x,appearance.bottom()+16,card_width,tile_height);
    Rect scale(stacked?x:name.right()+16,stacked?name.bottom()+16:name.y,card_width,tile_height);
    c.fill_rounded_rect(name,24,Color::from_hex(0xF2FAFF,.96f));
    c.draw_text("Your name",{name.x+20,name.y+18},20,Color::Black,FontWeight::ExtraBold);
    c.draw_text("Shown above your playlists.",{name.x+20,name.y+48},12,Color::Black,FontWeight::Light);
    Rect field(name.x+20,name.y+field_y,name.w-40,field_height);
    c.fill_rounded_rect(field,10,Color::White);
    c.stroke_rounded_rect(field,10,Color::from_hex(editing_name?0x0034FF:0x869FFF),editing_name?2:1);
    fitted(c,name_draft+(editing_name?"|":""),{field.x+10,field.y+10},16,field.w-20);
    add_hit(field,[this]{editing_name=true;select_name=true;});
    c.draw_text("Press Enter to save",{name.x+20,name.y+hint_y},11,Color::Black,FontWeight::Light);
    c.fill_rounded_rect(scale,24,Color::from_hex(0xF2FAFF,.96f));
    c.draw_text("Interface size",{scale.x+20,scale.y+18},20,Color::Black,FontWeight::ExtraBold);
    c.draw_text("Scale the whole interface.",{scale.x+20,scale.y+48},12,Color::Black,FontWeight::Light);
    int values[]={80,100,125,150}; float button_width=(scale.w-40)/4;
    for(int i=0;i<4;++i){
        Rect button(scale.x+20+i*button_width,scale.y+field_y,button_width-4,field_height);
        c.fill_rounded_rect(button,10,Color::from_hex(preferences.interface_percent==values[i]?0x0064DF:0xAAE4FF));
        c.draw_text(std::to_string(values[i])+"%",{button.x+button.w/2,button.y+12},12,preferences.interface_percent==values[i]?Color::White:Color::Black,FontWeight::Medium,TextAlign::Center);
        add_hit(button,[this,v=values[i]]{preferences.interface_percent=v;save_preferences();});
    }
    c.draw_text("Retina display quality stays automatic.",{scale.x+20,scale.y+hint_y},11,Color::Black,FontWeight::Light);
    if(!save_error.empty()) wrapped(c,save_error,{x,scale.bottom()+12,width,48},12);

}
void LibraryExperience::render_menu(Canvas& c) {
    auto r=menu_rect; r.x=std::clamp(r.x,rendered_bounds.x,rendered_bounds.right()-r.w);
    r.y=std::clamp(r.y,rendered_bounds.y,rendered_bounds.bottom()-r.h); menu_rect=r;
    c.fill_rounded_rect(r,12,Color::White); c.stroke_rounded_rect(r,12,Color::from_hex(0x869FFF),1);
    const char* labels[]={"Play","Shuffle","View Artist"};
    int count=navigation.current.page==LibraryPage::Album?3:2;
    for(int i=0;i<count;++i){ Rect row(r.x+4,r.y+4+i*34,r.w-8,34); c.draw_text(labels[i],{row.x+10,row.y+9},14,Color::Black,FontWeight::Medium);
        menu_hits.push_back({row,[this,i]{ menu_open=false; if(i==0)start(false,menu_track); if(i==1)start(true); if(i==2){auto a=library.album(navigation.current.id);if(a)open(LibraryPage::Artist,a->artist_id);} }});
    }
}
bool LibraryExperience::click(float x,float y) {
    if(menu_open){ for(auto& h:menu_hits) if(h.rect.contains(x,y)){auto action=h.action;action();return true;} menu_open=false;return true; }
    y-=5*(1-page_reveal.val);
    if(editing_name && !commit_name()) return true;
    editing_name=false;
    if(!rendered_bounds.contains(x,y))return false;
    if(scroll_rail.h>0 && scroll_rail.inset(-5,0).contains(x,y)){dragging_scroll=true;pointer_move(x,y);return true;}
    for(auto it=hits.rbegin();it!=hits.rend();++it) if(it->rect.contains(x,y)){auto action=it->action;action();return true;}
    return true;
}
bool LibraryExperience::key(SDL_Keycode key,SDL_Keymod modifiers) {
    if(!editing_name)return false;
    if(key==SDLK_ESCAPE){name_draft=preferences.display_name;editing_name=false;save_error="";return true;}
    if(key==SDLK_RETURN || key==SDLK_TAB){if(commit_name())editing_name=false;return true;}
    if(key==SDLK_V && (modifiers & (SDL_KMOD_GUI|SDL_KMOD_CTRL))){char* value=SDL_GetClipboardText();if(value){auto clean=trimmed(value);text(clean);SDL_free(value);}return true;}
    if(key==SDLK_A && (modifiers & (SDL_KMOD_GUI|SDL_KMOD_CTRL))){select_name=true;return true;}
    if(key==SDLK_BACKSPACE){if(select_name)name_draft.clear();else erase_character(name_draft);select_name=false;return true;}
    return true;
}
void LibraryExperience::text(const std::string& input){if(!editing_name)return;if(select_name)name_draft.clear();select_name=false;if(name_draft.size()+input.size()<=192)name_draft+=input;}
void LibraryExperience::pointer_move(float x, float y) {
    pointer={x,y-5*(1-page_reveal.val)};
    if(!dragging_scroll || scroll_rail.h<=scroll_thumb)return;
    float ratio=std::clamp((y-scroll_rail.y-scroll_thumb*.5f)/(scroll_rail.h-scroll_thumb),0.f,1.f);
    float offset=ratio*std::max(0.f,content_height-rendered_bounds.h);
    if(settings)settings_scroll=offset;else navigation.current.scroll=offset;
    scroll_position.snap(offset);
}
void LibraryExperience::scroll(float dy){menu_open=false;if(settings)settings_scroll=std::max(0.f,settings_scroll-dy*40);else navigation.current.scroll=std::max(0.f,navigation.current.scroll-dy*40);}
ArtworkPalette LibraryExperience::palette(const std::string& path) {
    if(auto i=palettes.find(path);i!=palettes.end())return i->second;
    ArtworkPalette result;
    SDL_Surface* raw=IMG_Load(path.c_str());
    if(raw){
        SDL_Surface* image=SDL_ConvertSurface(raw,SDL_PIXELFORMAT_RGBA32);SDL_DestroySurface(raw);
        if(image){
            struct Bucket{double r=0,g=0,b=0,weight=0;}; std::array<Bucket,12> buckets{};
            auto pixels=static_cast<unsigned char*>(image->pixels);
            for(int y=0;y<image->h;y+=std::max(1,image->h/48))for(int x=0;x<image->w;x+=std::max(1,image->w/48)){
                auto p=pixels+y*image->pitch+x*4;if(p[3]<128)continue;
                float r=p[0]/255.f,g=p[1]/255.f,b=p[2]/255.f,hi=std::max({r,g,b}),lo=std::min({r,g,b}),d=hi-lo,h=0;
                if(d>.01f){if(hi==r)h=std::fmod((g-b)/d+6,6.f);else if(hi==g)h=(b-r)/d+2;else h=(r-g)/d+4;}
                float saturation=hi>0?d/hi:0;
                if(saturation<.12f || hi<.08f)continue;
                auto& bucket=buckets[std::min(11,int(h*2))]; double weight=saturation*saturation*(.5f+hi);
                bucket.r+=r*weight;bucket.g+=g*weight;bucket.b+=b*weight;bucket.weight+=weight;
            }
            std::sort(buckets.begin(),buckets.end(),[](auto a,auto b){return a.weight>b.weight;});
            if(buckets[0].weight>0){auto color=[](Bucket b){return Color(float(b.r/b.weight),float(b.g/b.weight),float(b.b/b.weight));};result.primary=color(buckets[0]);result.secondary=buckets[1].weight>0?color(buckets[1]):result.primary;result.valid=true;}
            else {result.primary=Color::from_hex(0x65788A);result.secondary=Color::from_hex(0xA4ABB2);result.valid=true;}
            SDL_DestroySurface(image);
        }
    }
    palettes[path]=result;return result;
}
cairo_surface_t* LibraryExperience::background(const std::string& path) {
    if(auto i=backgrounds.find(path);i!=backgrounds.end())return i->second;
    if(backgrounds.size()>=8){auto i=backgrounds.begin();cairo_surface_destroy(i->second);backgrounds.erase(i);}
    auto p=palette(path);auto surface=cairo_image_surface_create(CAIRO_FORMAT_ARGB32,256,256);auto cr=cairo_create(surface);
    Color base=mix(p.primary,Color::White,.85f);cairo_set_source_rgb(cr,base.r,base.g,base.b);cairo_paint(cr);
    auto blob=[&](float x,float y,Color color,float radius){auto gradient=cairo_pattern_create_radial(x,y,0,x,y,radius);cairo_pattern_add_color_stop_rgba(gradient,0,color.r,color.g,color.b,.50);cairo_pattern_add_color_stop_rgba(gradient,1,color.r,color.g,color.b,0);cairo_set_source(cr,gradient);cairo_paint(cr);cairo_pattern_destroy(gradient);};
    blob(40,45,mix(p.primary,Color::White,.45f),190);blob(230,150,mix(p.secondary,Color::White,.45f),170);blob(100,280,mix(p.primary,Color::White,.45f),150);
    cairo_destroy(cr);backgrounds[path]=surface;return surface;
}
void LibraryExperience::update(float dt,const std::string& playing_cover,bool library_active) {
    playing_artwork=playing_cover;
    auto signature=settings?std::string("settings"):std::to_string(int(navigation.current.page))+navigation.current.id;
    if(signature!=last_route){scroll_position.snap(settings?settings_scroll:navigation.current.scroll);last_route=signature;}
    scroll_position.set_target(settings?settings_scroll:navigation.current.scroll);scroll_position.update(dt);
    page_reveal.update(dt);toggle_position.set_target(preferences.coverflow_colors?1.f:0.f);toggle_position.update(dt);
    auto route=(settings || !library_active)?LibraryRoute{}:navigation.current;
    auto key=preferences.coverflow_colors?theme_artwork(library,route,playing_cover):"";
    auto p=key.empty()?ArtworkPalette{}:palette(key);
    if(preferences.coverflow_colors && !p.valid && route.page==LibraryPage::Artist){auto a=library.artist(route.id);if(a && !a->albums.empty()){key=library.album(a->albums.front())->cover;p=palette(key);}}
    if(!p.valid)key="";
    if(key!=artwork_key){previous_key=artwork_key;previous_palette=current_palette;artwork_key=key;current_palette=p;theme_mix=0;}
    theme_mix=std::min(1.f,theme_mix+dt/.35f);
}
Color LibraryExperience::remap(const Color& c) const {
    // Theme UI blues only; image and SVG rasterization remain untouched.
    if(c.b<c.r+.12f || c.b<.45f)return c;
    auto transform=[&](ArtworkPalette p){if(!p.valid)return c;float luminance=.2126f*c.r+.7152f*c.g+.0722f*c.b;Color out;
        if(luminance<.38f)out=mix(p.primary,Color::Black,.55f);
        else out=mix(p.primary,Color::White,.52f+luminance*.42f);
        out.a=c.a;return out;};
    return mix(transform(previous_palette),transform(current_palette),theme_mix);
}
void LibraryExperience::render_background(Canvas& c,Rect b) {
    auto cr=c.cr();c.push_clip_rounded_rect(b,16);
    auto paint=[&](const std::string& key,float alpha){if(key.empty() || alpha<=0)return;cairo_save(cr);cairo_translate(cr,b.x,b.y);cairo_scale(cr,b.w/256,b.h/256);cairo_set_source_surface(cr,background(key),0,0);cairo_paint_with_alpha(cr,alpha);cairo_restore(cr);};
    paint(previous_key,1-theme_mix);paint(artwork_key,theme_mix);c.pop_clip();
}
}
