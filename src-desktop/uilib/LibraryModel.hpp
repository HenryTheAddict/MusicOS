#pragma once
#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <map>
#include <random>
#include <sstream>
#include <string>
#include <vector>

namespace MusicUI {
struct CatalogRecord;
inline std::string trimmed(std::string s) {
    const auto a = s.find_first_not_of(" \t\r\n");
    if (a == std::string::npos) return {};
    return s.substr(a, s.find_last_not_of(" \t\r\n") - a + 1);
}
inline void erase_character(std::string& text) {
    if (text.empty()) return;
    size_t i = text.size() - 1;
    while (i > 0 && (static_cast<unsigned char>(text[i]) & 0xc0) == 0x80) --i;
    text.erase(i);
}
struct Preferences {
    bool coverflow_colors = false;
    std::string display_name = "Henry";
    int interface_percent = 100;
    static bool valid_scale(int v) { return v == 80 || v == 100 || v == 125 || v == 150; }
    static bool valid_name(const std::string& s) {
        size_t characters=0; for(unsigned char c:s) if((c&0xc0)!=0x80)++characters;
        return !trimmed(s).empty() && characters<=48 && s.size() <= 192 && s.find_first_of("\r\n\t") == std::string::npos;
    }
    static Preferences load(const std::string& path) {
        Preferences p;
        std::ifstream f(path);
        std::string line;
        while (std::getline(f, line)) {
            std::istringstream input(line);
            std::string key; input >> key;
            if (key == "colors") { int v = -1; if (input >> v && (v == 0 || v == 1)) p.coverflow_colors = v == 1; }
            if (key == "scale") { int v = 0; if (input >> v && valid_scale(v)) p.interface_percent = v; }
            if (key == "name") { std::string v; if (input >> std::quoted(v) && valid_name(v)) p.display_name = trimmed(v); }
        }
        return p;
    }
    bool save(const std::string& path) const {
        if (path.empty()) return false;
        std::ofstream f(path + ".tmp", std::ios::trunc);
        f << "version 1\ncolors " << coverflow_colors << "\nscale " << interface_percent
          << "\nname " << std::quoted(display_name) << '\n';
        f.close();
        if (!f) return false;
        std::error_code error;
        std::filesystem::rename(path + ".tmp", path, error);
        return !error;
    }
};
struct DemoTrack { std::string id, album_id, title; int seconds = 180; };
struct DemoAlbum { std::string id, artist_id, title, artist, cover; std::vector<std::string> tracks; std::string genre="Alternative", catalog_id, store_url; };
struct DemoArtist { std::string id, name, portrait, biography; std::vector<std::string> albums; std::string catalog_id, store_url; };
struct DemoLibrary {
    std::vector<DemoAlbum> albums;
    std::vector<DemoArtist> artists;
    std::map<std::string, DemoTrack> tracks;
    DemoLibrary();
    void import_catalog(const std::vector<CatalogRecord>& records);
    const DemoAlbum* album(const std::string& id) const {
        for (const auto& a : albums) if (a.id == id) return &a;
        return nullptr;
    }
    const DemoArtist* artist(const std::string& id) const {
        for (const auto& a : artists) if (a.id == id) return &a;
        return nullptr;
    }
    std::vector<std::string> artist_tracks(const std::string& id) const {
        std::vector<std::string> result;
        if (auto a = artist(id)) for (auto& aid : a->albums) {
            auto b = album(aid); result.insert(result.end(), b->tracks.begin(), b->tracks.end());
        }
        return result;
    }
};
enum class LibraryPage { Grid, Album, Artist };
struct LibraryRoute { LibraryPage page = LibraryPage::Grid; std::string id; float scroll = 0; };
struct LibraryNavigation {
    LibraryRoute current;
    std::vector<LibraryRoute> history;
    void open(LibraryPage page, const std::string& id) { history.push_back(current); current = {page, id, 0}; }
    bool back() { if (history.empty()) return false; current = history.back(); history.pop_back(); return true; }
    void grid() { current = {}; history.clear(); }
};
struct DemoSequence {
    std::vector<std::string> ids, ordered_ids;
    bool shuffled = false;
    size_t position = 0;
    void start(std::vector<std::string> list, bool shuffle = false, const std::string& first = {}) {
        ordered_ids = list; ids = std::move(list); position = 0; shuffled = shuffle;
        if (shuffle) { std::mt19937 random(std::random_device{}()); std::shuffle(ids.begin(), ids.end(), random); }
        if (!first.empty()) { auto i = std::find(ids.begin(), ids.end(), first); if (i != ids.end()) position = size_t(i - ids.begin()); }
    }
    void set_shuffle(bool enabled) {
        auto selected=current();
        start(ordered_ids,enabled,selected);
    }
    std::string current() const { return ids.empty() ? "" : ids[position]; }
    void next(int delta) { if (!ids.empty()) position = (position + ids.size() + delta) % ids.size(); }
};
inline std::string theme_artwork(const DemoLibrary& lib, const LibraryRoute& route, const std::string& playing_cover) {
    if (route.page == LibraryPage::Album) { if (auto a = lib.album(route.id)) return a->cover; }
    if (route.page == LibraryPage::Artist) { if (auto a = lib.artist(route.id)) {
        if (!a->portrait.empty()) return a->portrait;
        if (!a->albums.empty()) return lib.album(a->albums.front())->cover;
    }}
    return playing_cover;
}
inline std::string duration_text(int seconds) { return std::to_string(seconds / 60) + ":" + (seconds % 60 < 10 ? "0" : "") + std::to_string(seconds % 60); }
}
