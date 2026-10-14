#include "../Components.hpp"
#include "../LibraryModel.hpp"
#include "../MusicSearchService.hpp"
#include <set>
#include <cmath>

namespace MusicUI {
namespace {
struct Genre {const char* name;const char* id;};
const Genre genres[]={{"Alternative","20"},{"Electronic","7"},{"Rock","21"},{"Pop","14"},{"Hip-Hop/Rap","18"},{"R&B/Soul","15"},{"Jazz","11"},{"Classical","5"},{"Country","6"},{"Dance","17"}};
std::string lower(std::string text){for(auto& c:text)c=char(std::tolower(static_cast<unsigned char>(c)));return text;}
void label(Canvas& c,std::string text,Point p,float size,float width,FontWeight weight=FontWeight::Medium,TextAlign align=TextAlign::Left){if(c.measure_text(text,size,weight).w>width){while(!text.empty() && c.measure_text(text+"…",size,weight).w>width)erase_character(text);text+="…";}c.draw_text(text,p,size,Color::Black,weight,align);}
}
AlbumGrid::AlbumGrid(const Rect& b):Widget(b){for(auto& anim:card_scale)anim=SpringAnim(1,650,51);set_category(0);}
void AlbumGrid::set_category(int value){category=value;genre_filter.clear();search_query.clear();search_status.clear();active_request.clear();search_active=false;is_searching=false;pending_search=false;selected_index=hovered_index=-1;scroll_y=target_scroll_y=0;rebuild();}
void AlbumGrid::rebuild(){
    static const DemoLibrary fallback;const auto& source=library?*library:fallback;
    using Kind=AlbumCard::Kind;all_cards.clear();
    if(category==0 || (category==2 && !genre_filter.empty())){
        for(const auto& a:source.albums)if(genre_filter.empty() || a.genre==genre_filter)all_cards.push_back({a.title,a.artist,a.cover,a.id,Kind::Album});
    }else if(category==1){
        for(const auto& a:source.artists){auto cover=a.portrait;if(cover.empty() && !a.albums.empty())cover=source.album(a.albums.front())->cover;all_cards.push_back({a.name,std::to_string(a.albums.size())+" albums",cover,a.id,Kind::Artist});}
    }else if(category==2){
        for(auto genre:genres){int count=0;std::string cover;for(const auto& album:source.albums)if(album.genre==genre.name){++count;if(cover.empty())cover=album.cover;}all_cards.push_back({genre.name,std::to_string(count)+" albums",cover,genre.name,Kind::Genre});}
    }else{
        for(const auto& a:source.albums)for(const auto& id:a.tracks){const auto& t=source.tracks.at(id);all_cards.push_back({t.title,a.artist,a.cover,id,Kind::Song});}
    }
    auto query=lower(search_query);cards.clear();for(const auto& card:all_cards)if(query.empty() || lower(card.title+" "+card.artist).find(query)!=std::string::npos)cards.push_back(card);
}
void AlbumGrid::set_search_query(const std::string& query){search_query=query;scroll_y=target_scroll_y=0;rebuild();pending_search=query.size()>=2;search_debounce=.5f;search_status=query.empty()?"":"Press Enter to search the catalog";}
void AlbumGrid::tick_search(float dt){if(!pending_search)return;search_debounce-=dt;if(search_debounce<=0 && (last_request==0 || SDL_GetTicks()-last_request>=3000)){pending_search=false;trigger_search(search_query);}}
void AlbumGrid::trigger_search(const std::string& query){if(query.empty())return;pending_search=false;is_searching=true;active_request=query;search_status="Searching Apple Music catalog…";last_request=SDL_GetTicks();if(on_search_catalog)on_search_catalog(query,category,{});}
void AlbumGrid::open_genre(std::string genre){genre_filter=genre;search_query.clear();scroll_y=target_scroll_y=0;rebuild();is_searching=true;active_request=genre;search_status="Loading catalog albums…";for(auto entry:genres)if(genre==entry.name)if(on_search_catalog)on_search_catalog(genre,2,entry.id);}
void AlbumGrid::apply_catalog(const CatalogBatch& batch){rebuild();if(batch.context.empty() && batch.category==category && batch.query==active_request){is_searching=false;search_status=batch.error.empty()?(batch.records.empty()?"No catalog matches":"Apple Music catalog · "+std::to_string(batch.records.size())+" results"):batch.error;} }
void AlbumGrid::ingest_pending_results(){for(auto& batch:MusicSearchService::instance().poll())if(on_catalog_results)on_catalog_results(batch);}
void AlbumGrid::handle_wheel(float delta){target_scroll_y=std::clamp(target_scroll_y-delta*32.f,0.f,max_scroll_y);}
bool AlbumGrid::handle_key_down(SDL_Keycode key){if(!search_active)return false;if(key==SDLK_RETURN || key==SDLK_KP_ENTER){trigger_search(search_query);return true;}if(key==SDLK_BACKSPACE){erase_character(search_query);set_search_query(search_query);return true;}if(key==SDLK_ESCAPE){search_active=false;return true;}return true;}
bool AlbumGrid::handle_text_input(const char* text){if(!search_active || !text)return false;set_search_query(search_query+text);return true;}
void AlbumGrid::render_self(Canvas& c){
    content_hits.clear();Rect header(bounds.x,bounds.y,bounds.w,32);c.fill_rounded_rect(header,16,Color::from_hex(0xD6EDFD));
    const char* names[]={"Albums","Artists","Genres","Songs"};std::string heading=genre_filter.empty()?names[category]:"‹ All genres · "+genre_filter;
    label(c,heading,{header.x+12,header.y+9},14,std::max(100.f,header.w-280),FontWeight::DemiBold);
    float search_w=std::min(260.f,header.w-190);Rect search(header.right()-search_w,header.y,search_w,32);
    if(show_search_bar){c.fill_rounded_rect(search,12,Color::from_hex(search_active?0xFFFFFF:0xAAE4FF));c.draw_svg("assets/icons/sketch/search.svg",{search.x+6,search.y+4,24,24});label(c,search_query.empty()?"Search library and catalog":search_query,{search.x+36,search.y+10},12,search.w-48);if(search_active)c.stroke_rounded_rect(search,12,Color::from_hex(0x0064DF),1.5f);}
    Rect body(bounds.x,bounds.y+36,bounds.w-3,bounds.h-36);c.fill_rounded_rect(body,16,Color::from_hex(0x00AEFF));c.stroke_rounded_rect(body,16,Color::from_hex(0x0018FF),1);
    float notice=search_status.empty()?0.f:32.f;
    c.push_clip_rounded_rect(body,16);
    float top=body.y+20-scroll_y+notice;
    if(!search_status.empty())label(c,search_status,{body.x+20,body.y+13-scroll_y},11,body.w-42,FontWeight::Medium);
    if(cards.empty()){c.draw_text(is_searching?"Searching…":"Nothing here yet",{body.x+24,body.y+76},24,Color::Black,FontWeight::DemiBold);label(c,is_searching?"Catalog results will appear here.":"Try a name, title, or another genre.",{body.x+24,body.y+113},14,body.w-48,FontWeight::Light);}
    if(category==3){
        const auto& source=*library;float row_h=56;float usable=body.w-56;
        float title_w=usable*.44f,artist_w=usable*.24f,album_w=usable*.25f;
        c.fill_rounded_rect({body.x+16,top,body.w-40,28},10,Color::from_hex(0xD6EDFD,.72f));
        c.draw_text("Title",{body.x+68,top+8},11,Color::Black,FontWeight::DemiBold);
        c.draw_text("Artist",{body.x+68+title_w,top+8},11,Color::Black,FontWeight::DemiBold);
        c.draw_text("Album",{body.x+68+title_w+artist_w,top+8},11,Color::Black,FontWeight::DemiBold);
        for(size_t i=0;i<cards.size();++i){
            auto& card=cards[i];Rect row(body.x+16,top+36+i*row_h,body.w-40,row_h-4);if(row.bottom()<body.y || row.y>body.bottom())continue;
            auto track=source.tracks.find(card.id);if(track==source.tracks.end())continue;auto album=source.album(track->second.album_id);
            c.fill_rounded_rect(row,12,Color::from_hex(card.id==playing_track?0xD6EDFD:0x0034FF,card.id==playing_track?.95f:(hovered_index==int(i)?.20f:.10f)));
            c.draw_image(card.cover_path,{row.x+8,row.y+8,36,36},6);
            label(c,card.title,{row.x+52,row.y+17},13,title_w-12,FontWeight::DemiBold);
            float artist_x=row.x+52+title_w,album_x=artist_x+artist_w;
            label(c,card.artist,{artist_x,row.y+18},12,artist_w-12,FontWeight::Light);
            label(c,album?album->title:"",{album_x,row.y+18},12,std::max(30.f,album_w-45),FontWeight::Light);
            c.draw_text(duration_text(track->second.seconds),{row.right()-8,row.y+18},11,Color::Black,FontWeight::Light,TextAlign::Right);
            content_hits.push_back({row,[this,i]{selected_index=int(i);if(on_album_select)on_album_select(int(i));}});
            if(album)content_hits.push_back({{artist_x,row.y,artist_w-8,row.h},[this,id=album->artist_id]{if(on_artist_select)on_artist_select(id);}});
        }
        max_scroll_y=std::max(0.f,notice+76+cards.size()*row_h-body.h);
    }else{
        int cols=std::max(3,int((body.w-42)/166));float cell=(body.w-42)/cols;float cover=std::min(128.f,cell-18);float row_h=cover+48;
        for(size_t i=0;i<cards.size();++i){
            const auto& card=cards[i];float x=body.x+21+float(i%cols)*cell,y=top+float(i/cols)*row_h;
            Rect card_rect(x,y,cell-8,row_h-8);if(card_rect.bottom()<body.y || y>body.bottom())continue;
            float center=x+(cell-8)*.5f;
            if(hovered_index==int(i))c.fill_rounded_rect(card_rect,16,Color::from_hex(0xD6EDFD,.24f));
            Rect art(center-cover*.5f,y+4,cover,cover);float radius=card.kind==AlbumCard::Kind::Artist?cover*.5f:16;
            if(!card.cover_path.empty())c.draw_image(card.cover_path,art,radius);
            else {c.fill_rounded_rect(art,radius,Color::from_hex(0xD6EDFD,.65f));c.draw_svg(card.kind==AlbumCard::Kind::Artist?"assets/icons/sketch/noARTIST.svg":"assets/icons/sketch/noCOVER.svg",art.inset(24,24));}
            if(card.kind==AlbumCard::Kind::Genre){c.fill_rounded_rect({x,y+cover-30,cell-8,34},10,Color::from_hex(0xD6EDFD,.94f));label(c,card.title,{center,y+cover-20},14,cell-18,FontWeight::DemiBold,TextAlign::Center);}
            else label(c,card.title,{center,y+cover+10},13,cell-12,FontWeight::DemiBold,TextAlign::Center);
            label(c,card.artist,{center,y+cover+28},10,cell-12,FontWeight::Light,TextAlign::Center);
            content_hits.push_back({card_rect,[this,i]{selected_index=int(i);if(cards[i].kind==AlbumCard::Kind::Genre)open_genre(cards[i].id);else if(on_album_select)on_album_select(int(i));}});
        }
        int rows=int((cards.size()+cols-1)/cols);max_scroll_y=std::max(0.f,notice+40+rows*row_h-body.h);
    }
    target_scroll_y=std::clamp(target_scroll_y,0.f,max_scroll_y);scroll_y=std::clamp(scroll_y,0.f,max_scroll_y);c.pop_clip();
    if(max_scroll_y>0){float track_h=std::max(40.f,body.h-40),thumb=std::max(28.f,track_h*body.h/(body.h+max_scroll_y));float thumb_y=body.y+20+(track_h-thumb)*scroll_y/max_scroll_y;c.fill_rounded_rect({body.right()-10,body.y+20,6,track_h},3,Color::from_hex(0x0034FF,.28f));c.fill_rounded_rect({body.right()-10,thumb_y,6,thumb},3,Color::White);}
}
bool AlbumGrid::handle_mouse_down(float x,float y,int button){
    if(button!=SDL_BUTTON_LEFT || !bounds.contains(x,y))return false;
    float search_w=std::min(260.f,bounds.w-190);Rect search(bounds.right()-search_w,bounds.y,search_w,32);
    if(show_search_bar && search.contains(x,y)){search_active=true;return true;}search_active=false;
    if(!genre_filter.empty() && Rect(bounds.x,bounds.y,bounds.w-search_w,32).contains(x,y)){set_category(2);return true;}
    Rect body(bounds.x,bounds.y+36,bounds.w-3,bounds.h-36);
    if(!body.contains(x,y))return true;
    if(x>body.right()-18 && max_scroll_y>0){is_dragging_scroll=true;drag_start_y=y;drag_start_scroll=target_scroll_y;return true;}
    for(auto it=content_hits.rbegin();it!=content_hits.rend();++it)if(it->rect.contains(x,y)){auto action=it->action;action();return true;}
    return true;
}
bool AlbumGrid::handle_mouse_move(float x,float y){
    if(is_dragging_scroll){float track_h=std::max(40.f,bounds.h-76);target_scroll_y=std::clamp(drag_start_scroll+(y-drag_start_y)*(max_scroll_y+track_h)/track_h,0.f,max_scroll_y);scroll_y=target_scroll_y;return true;}
    hovered_index=-1;for(size_t i=0;i<content_hits.size();++i)if(content_hits[i].rect.contains(x,y)){hovered_index=int(category==3?i/2:i);break;}return hovered_index>=0;
}
bool AlbumGrid::handle_mouse_up(float,float,int){bool was=is_dragging_scroll;is_dragging_scroll=false;return was;}
}
