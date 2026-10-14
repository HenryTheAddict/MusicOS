#pragma once
#include <SDL3/SDL.h>
#include <atomic>
#include <condition_variable>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace MusicUI {
struct CatalogRecord {
    std::string kind, artist_id, album_id, track_id, title, artist, album, genre, artwork_url, cover, store_url;
    int seconds=0;
};
struct CatalogBatch {
    int category=0;
    std::string query, context, error;
    std::vector<CatalogRecord> records;
};
class MusicSearchService {
public:
    static MusicSearchService& instance();
    MusicSearchService();
    ~MusicSearchService();
    void search_async(const std::string& query,int category=0,const std::string& genre_id={});
    void lookup_async(const std::string& id,bool artist);
    std::vector<CatalogBatch> poll();
    void shutdown();
    static std::vector<CatalogRecord> parse(const std::string& json);
private:
    struct Request {std::string url,query,context;int category=0;uint64_t generation=0;};
    void submit(Request request);
    void work();
    std::string fetch(const std::string& url,bool& ok);
    std::string cover_path(const CatalogRecord& record,const Request& request);
    std::mutex mutex;
    std::condition_variable condition;
    std::thread worker;
    std::atomic<bool> stopped{false};
    std::atomic<uint64_t> generation{0};
    Request pending;
    bool requested=false;
    std::vector<CatalogBatch> completed;
    std::string cache_dir;
};
}
