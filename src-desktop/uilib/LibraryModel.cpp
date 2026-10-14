#include "LibraryModel.hpp"
#include "MusicSearchService.hpp"
namespace MusicUI {
DemoLibrary::DemoLibrary() {
    struct Seed { const char* title; const char* artist; const char* cover; };
    const Seed seeds[] = {
            { "Plastic Beach", "Gorillaz", "assets/covers/Beach.png" },
            { "Demon Days", "Gorillaz", "assets/covers/DemonDays.png" },
            { "Random Access Memories", "Daft Punk", "assets/covers/RAM.png" },
            { "Discovery", "Daft Punk", "assets/covers/Discovery.png" },
            { "Currents", "Tame Impala", "assets/covers/Currents.png" },
            { "The Slow Rush", "Tame Impala", "assets/covers/SlowRush.png" },
            { "Night Visions", "Imagine Dragons", "assets/covers/NightVisions.png" },
            { "AM", "Arctic Monkeys", "assets/covers/Overcast.png" },
            { "Torches", "Foster the People", "assets/covers/Baltimore.png" },
            { "Melophobia", "Cage The Elephant", "assets/covers/Luck.png" },
            { "Vessel", "Twenty One Pilots", "assets/covers/Pier.png" },
            { "Parachutes", "Coldplay", "assets/covers/Lake.png" },
            { "A Rush of Blood", "Coldplay", "assets/covers/Crowd.png" },
            { "Dark Side of the Moon", "Pink Floyd", "assets/covers/tritower.png" },
            { "Channel Orange", "Frank Ocean", "assets/covers/Clouds.png" },
            { "IGOR", "Tyler, The Creator", "assets/covers/Monk.png" },
            { "After Hours", "The Weeknd", "assets/covers/Lama.png" },
            { "OK Computer", "Radiohead", "assets/covers/Cave.png" },
            { "In Rainbows", "Radiohead", "assets/covers/Sand.png" },
            { "Blurryface", "Twenty One Pilots", "assets/covers/Stop.png" },
            { "Social Cues", "Cage The Elephant", "assets/covers/Hungry.png" },
            { "Supermodel", "Foster the People", "assets/covers/butterfly.png" },
            { "Whatever People Say", "Arctic Monkeys", "assets/covers/Rocks.png" },
            { "Smoke + Mirrors", "Imagine Dragons", "assets/covers/Towers.png" },
            { "Lonerism", "Tame Impala", "assets/covers/Hills.png" },
            { "Homework", "Daft Punk", "assets/covers/Apart.png" },
            { "The Now Now", "Gorillaz", "assets/covers/sky.png" },
            { "Favorite Worst Nightmare", "Arctic Monkeys", "assets/covers/Vacate.png" },
        {"Hurry Up, We're Dreaming", "M83", "assets/covers/sky.png"},
        {"Fantasy", "M83", "assets/covers/Clouds.png"},
        {"Junk", "M83", "assets/covers/Sand.png"}
    };
    for (const auto& seed : seeds) {
        auto key = [](std::string value) {
            for (char& c : value) { if (c >= 'A' && c <= 'Z') c += 32; else if (!((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))) c = '-'; }
            return value;
        };
        std::string artist_id = key(seed.artist);
        auto existing = std::find_if(artists.begin(), artists.end(), [&](const DemoArtist& a) { return a.id == artist_id; });
        if (existing == artists.end()) { artists.push_back({artist_id, seed.artist, "", "", {},{}, {}}); existing = artists.end() - 1; }
        DemoAlbum album{artist_id + "/" + key(seed.title), artist_id, seed.title, seed.artist, seed.cover, {},"Alternative",{}, {}};
        std::vector<std::string> titles;
        if (album.title == "Plastic Beach") titles = {"Orchestral Intro", "Welcome to the World of the Plastic Beach", "White Flag", "Rhinestone Eyes", "Stylo", "Superfast Jellyfish", "Empire Ants", "Glitter Freeze", "Some Kind of Nature", "On Melancholy Hill", "Broken", "Sweepstakes", "Plastic Beach", "To Binge", "Cloud of Unknowing", "Pirate Jet"};
        else if (album.title == "Hurry Up, We're Dreaming") titles = {"Intro", "Midnight City", "Reunion", "Where the Boats Go", "Wait", "Raconte-Moi Une Histoire"};
        else titles = {"Opening • Demo", "Second Light • Demo", "A Long Way Home Through the City at Night • Demo", "Interlude • Demo", "Afterglow • Demo", "Closing • Demo"};
        for (size_t n = 0; n < titles.size(); ++n) {
            auto id = album.id + "/" + std::to_string(n + 1);
            tracks.emplace(id, DemoTrack{id, album.id, titles[n], 150 + int(n % 5) * 23});
            album.tracks.push_back(id);
        }
        existing->albums.push_back(album.id);
        if(seed.artist==std::string("Daft Punk") || seed.artist==std::string("M83"))album.genre="Electronic";
        else if(seed.artist==std::string("Pink Floyd") || seed.artist==std::string("Arctic Monkeys") || seed.artist==std::string("Imagine Dragons"))album.genre="Rock";
        else if(seed.artist==std::string("Frank Ocean") || seed.artist==std::string("The Weeknd"))album.genre="R&B/Soul";
        else if(seed.artist==std::string("Tyler, The Creator"))album.genre="Hip-Hop/Rap";
        albums.push_back(std::move(album));
    }
    for (auto& a : artists) if (a.name == "M83") {
        a.portrait = "assets/artists/m83.png";
        a.biography = "M83 is a French electronic music project formed in Antibes in 1999. Initially a duo of multi-instrumentalists Nicolas Fromageau and Anthony Gonzalez, it is now led by Gonzalez.";
    }
}
}

namespace MusicUI {
void DemoLibrary::import_catalog(const std::vector<CatalogRecord>& records){
    for(const auto& record:records){
        auto artist=std::find_if(artists.begin(),artists.end(),[&](const DemoArtist& a){return a.catalog_id==record.artist_id || a.name==record.artist;});
        if(artist==artists.end()){artists.push_back({"apple/artist/"+record.artist_id,record.artist,{},{},{},record.artist_id,{}});artist=artists.end()-1;}
        artist->catalog_id=record.artist_id;artist->store_url="https://music.apple.com/us/artist/"+record.artist_id;
        if(record.album_id.empty())continue;
        std::string id="apple/album/"+record.album_id;
        auto album=std::find_if(albums.begin(),albums.end(),[&](const DemoAlbum& a){return a.id==id;});
        if(album==albums.end()){albums.push_back({id,artist->id,record.album,artist->name,record.cover,{},record.genre,record.album_id,{}});album=albums.end()-1;}
        album->store_url="https://music.apple.com/us/album/"+record.album_id;
        if(!record.cover.empty())album->cover=record.cover;
        if(!record.genre.empty())album->genre=record.genre;
        if(std::find(artist->albums.begin(),artist->albums.end(),id)==artist->albums.end())artist->albums.push_back(id);
        if(record.kind=="track" && !record.track_id.empty()){
            auto track_id="apple/track/"+record.track_id;
            tracks[track_id]={track_id,id,record.title,std::max(1,record.seconds)};
            if(std::find(album->tracks.begin(),album->tracks.end(),track_id)==album->tracks.end())album->tracks.push_back(track_id);
        }
    }
}
}
