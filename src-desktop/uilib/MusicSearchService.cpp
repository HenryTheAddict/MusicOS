#include "MusicSearchService.hpp"
#include <curl/curl.h>
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-function"
#endif
#include <json-c/json.h>
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
#include <filesystem>
#include <fstream>

namespace MusicUI {
namespace {
size_t append(void* data,size_t size,size_t count,void* user){auto& text=*static_cast<std::string*>(user);size_t bytes=size*count;if(text.size()+bytes>12*1024*1024)return 0;text.append(static_cast<char*>(data),bytes);return bytes;}
std::string value(json_object* object,const char* key){json_object* item=nullptr;if(!json_object_object_get_ex(object,key,&item))return {};return json_object_get_string(item);}
int number(json_object* object,const char* key){json_object* item=nullptr;return json_object_object_get_ex(object,key,&item)?json_object_get_int(item):0;}
std::string encode(const std::string& query){auto curl=curl_easy_init();if(!curl)return {};auto escaped=curl_easy_escape(curl,query.c_str(),int(query.size()));std::string result=escaped?escaped:"";if(escaped)curl_free(escaped);curl_easy_cleanup(curl);return result;}
}
MusicSearchService& MusicSearchService::instance(){static MusicSearchService service;return service;}
MusicSearchService::MusicSearchService(){curl_global_init(CURL_GLOBAL_DEFAULT);char* root=SDL_GetPrefPath("MusicOS","MusicOSDesktop");cache_dir=root?std::string(root)+"catalog/":"out/catalog/";SDL_free(root);std::error_code error;std::filesystem::create_directories(cache_dir,error);worker=std::thread([this]{work();});}
MusicSearchService::~MusicSearchService(){shutdown();curl_global_cleanup();}
void MusicSearchService::shutdown(){stopped=true;++generation;condition.notify_all();if(worker.joinable())worker.join();}
void MusicSearchService::submit(Request request){request.generation=++generation;{std::lock_guard<std::mutex> lock(mutex);pending=std::move(request);requested=true;}condition.notify_one();}
void MusicSearchService::search_async(const std::string& query,int category,const std::string& genre_id){
    std::string entity=category==1?"musicArtist":(category==3 || !genre_id.empty())?"song":"album";
    auto url="https://itunes.apple.com/search?media=music&country=us&entity="+entity+"&limit=30&term="+encode(genre_id.empty()?query:genre_id);
    if(!genre_id.empty())url+="&attribute=genreIndex";
    submit({url,query,{},category,0});
}
void MusicSearchService::lookup_async(const std::string& id,bool artist){
    auto slash=id.find_last_of('/');auto numeric=slash==std::string::npos?id:id.substr(slash+1);
    if(numeric.find_first_not_of("0123456789")!=std::string::npos || numeric.empty())return;
    submit({"https://itunes.apple.com/lookup?id="+numeric+"&entity="+(artist?"album":"song")+"&limit=50&country=us",{},id,artist?1:0,0});
}
std::vector<CatalogRecord> MusicSearchService::parse(const std::string& text){
    std::vector<CatalogRecord> records;auto root=json_tokener_parse(text.c_str());if(!root)return records;
    json_object* results=nullptr;
    if(json_object_object_get_ex(root,"results",&results) && json_object_is_type(results,json_type_array)){
        for(size_t i=0;i<json_object_array_length(results);++i){
            auto item=json_object_array_get_idx(results,i);CatalogRecord r;
            r.kind=value(item,"wrapperType");r.artist=value(item,"artistName");r.artist_id=value(item,"artistId");r.album_id=value(item,"collectionId");r.track_id=value(item,"trackId");
            r.album=value(item,"collectionName");r.title=r.kind=="artist"?r.artist:r.kind=="track"?value(item,"trackName"):r.album;
            r.genre=value(item,"primaryGenreName");r.artwork_url=value(item,"artworkUrl100");r.seconds=number(item,"trackTimeMillis")/1000;
            r.store_url=value(item,r.kind=="track"?"trackViewUrl":r.kind=="artist"?"artistLinkUrl":"collectionViewUrl");
            auto at=r.artwork_url.find("100x100bb");if(at!=std::string::npos)r.artwork_url.replace(at,9,"300x300bb");
            if(!r.artist_id.empty() && !r.title.empty())records.push_back(std::move(r));
        }
    }
    json_object_put(root);return records;
}
std::string MusicSearchService::fetch(const std::string& url,bool& ok){
    ok=false;std::string data;auto curl=curl_easy_init();if(!curl)return data;
    curl_easy_setopt(curl,CURLOPT_URL,url.c_str());curl_easy_setopt(curl,CURLOPT_FOLLOWLOCATION,1L);
    curl_easy_setopt(curl,CURLOPT_WRITEFUNCTION,append);curl_easy_setopt(curl,CURLOPT_WRITEDATA,&data);
    curl_easy_setopt(curl,CURLOPT_USERAGENT,"MusicOS/1.0");curl_easy_setopt(curl,CURLOPT_TIMEOUT,8L);curl_easy_setopt(curl,CURLOPT_CONNECTTIMEOUT,4L);
    curl_easy_setopt(curl,CURLOPT_NOPROGRESS,0L);curl_easy_setopt(curl,CURLOPT_XFERINFOFUNCTION,+[](void* user,curl_off_t,curl_off_t,curl_off_t,curl_off_t)->int{return static_cast<MusicSearchService*>(user)->stopped?1:0;});curl_easy_setopt(curl,CURLOPT_XFERINFODATA,this);
    auto status=curl_easy_perform(curl);long code=0;curl_easy_getinfo(curl,CURLINFO_RESPONSE_CODE,&code);curl_easy_cleanup(curl);ok=status==CURLE_OK && code>=200 && code<300;return data;
}
std::string MusicSearchService::cover_path(const CatalogRecord& record,const Request& request){
    if(record.artwork_url.empty())return {};
    std::string path=cache_dir+"cover-"+record.album_id+".jpg";
    if(std::filesystem::is_regular_file(path))return path;
    if(stopped || generation!=request.generation)return {};
    bool ok;auto bytes=fetch(record.artwork_url,ok);if(!ok || bytes.empty())return {};
    std::ofstream file(path,std::ios::binary);file.write(bytes.data(),std::streamsize(bytes.size()));return file?path:"";
}
void MusicSearchService::work(){
    while(!stopped){
        Request request;{std::unique_lock<std::mutex> lock(mutex);condition.wait(lock,[this]{return stopped || requested;});if(stopped)return;request=pending;requested=false;}
        bool ok;std::string cache=cache_dir+"query-"+std::to_string(std::hash<std::string>{}(request.url))+".json";
        std::string data;std::error_code error;auto time=std::filesystem::last_write_time(cache,error);
        if(!error && std::filesystem::file_time_type::clock::now()-time<std::chrono::hours(24)){std::ifstream file(cache);data.assign(std::istreambuf_iterator<char>(file),{});ok=true;}
        else {data=fetch(request.url,ok);if(ok){std::ofstream file(cache);file<<data;}}
        CatalogBatch batch;batch.category=request.category;batch.query=request.query;batch.context=request.context;
        auto publish=[&]{if(stopped || generation!=request.generation)return;{std::lock_guard<std::mutex> lock(mutex);completed.push_back(batch);}if(SDL_WasInit(SDL_INIT_VIDEO)){SDL_Event event{};event.type=SDL_EVENT_USER;SDL_PushEvent(&event);}};
        if(ok){
            batch.records=parse(data);publish();
            int downloaded=0;
            for(auto& record:batch.records){
                if(stopped || generation!=request.generation)break;
                if(record.artwork_url.empty())continue;
                record.cover=cover_path(record,request);
                if(++downloaded%4==0)publish();
                if(downloaded>=12)break;
            }
            publish();
        }else{batch.error="Catalog unavailable. Check your connection and press Enter to retry.";publish();}
    }
}
std::vector<CatalogBatch> MusicSearchService::poll(){std::lock_guard<std::mutex> lock(mutex);auto out=std::move(completed);completed.clear();return out;}
}
