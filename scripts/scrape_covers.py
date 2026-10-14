#!/usr/bin/env python3
import json
import os
import urllib.parse
import urllib.request
import sys

ALBUMS = [
    ("Gorillaz", "Plastic Beach", "assets/covers/Beach.png"),
    ("Gorillaz", "Demon Days", "assets/covers/DemonDays.png"),
    ("Daft Punk", "Random Access Memories", "assets/covers/RAM.png"),
    ("Daft Punk", "Discovery", "assets/covers/Discovery.png"),
    ("Tame Impala", "Currents", "assets/covers/Currents.png"),
    ("Tame Impala", "The Slow Rush", "assets/covers/SlowRush.png"),
    ("Imagine Dragons", "Night Visions", "assets/covers/NightVisions.png"),
    ("Arctic Monkeys", "AM", "assets/covers/AM.png"),
    ("Foster the People", "Torches", "assets/covers/Torches.png"),
    ("Cage The Elephant", "Melophobia", "assets/covers/Melophobia.png"),
    ("Twenty One Pilots", "Vessel", "assets/covers/Vessel.png"),
    ("Coldplay", "Parachutes", "assets/covers/Parachutes.png"),
    ("Radiohead", "OK Computer", "assets/covers/OKComputer.png"),
    ("Radiohead", "Kid A", "assets/covers/KidA.png"),
    ("Pink Floyd", "The Dark Side of the Moon", "assets/covers/DarkSide.png"),
    ("Frank Ocean", "Blonde", "assets/covers/Blonde.png"),
    ("Tyler, The Creator", "IGOR", "assets/covers/IGOR.png"),
    ("The Weeknd", "After Hours", "assets/covers/AfterHours.png"),
    ("Daft Punk", "Homework", "assets/covers/Homework.png"),
    ("Gorillaz", "The Now Now", "assets/covers/TheNowNow.png"),
    ("Tame Impala", "Lonerism", "assets/covers/Lonerism.png"),
    ("Coldplay", "A Rush of Blood to the Head", "assets/covers/RushOfBlood.png"),
    ("Arctic Monkeys", "Favourite Worst Nightmare", "assets/covers/FWN.png"),
    ("Foster the People", "Supermodel", "assets/covers/Supermodel.png")
]

def scrape_cover(artist, album, dest_path):
    query = urllib.parse.quote(f"{artist} {album}")
    url = f"https://itunes.apple.com/search?term={query}&entity=album&limit=1"
    req = urllib.request.Request(url, headers={'User-Agent': 'MusicOS/1.0'})
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            data = json.loads(response.read().decode('utf-8'))
            if data.get('resultCount', 0) > 0:
                artwork_url = data['results'][0].get('artworkUrl100', '')
                if artwork_url:
                    # Upgrade to 600x600 high-res
                    hires_url = artwork_url.replace('100x100bb', '600x600bb')
                    print(f"Downloading {artist} - {album} from {hires_url}...")
                    art_req = urllib.request.Request(hires_url, headers={'User-Agent': 'MusicOS/1.0'})
                    with urllib.request.urlopen(art_req, timeout=10) as art_resp:
                        os.makedirs(os.path.dirname(dest_path), exist_ok=True)
                        with open(dest_path, 'wb') as f:
                            f.write(art_resp.read())
                        print(f" Saved to {dest_path}")
                        return True
    except Exception as e:
        print(f"Error scraping {artist} - {album}: {e}")
    return False

def main():
    print(f"Scraping {len(ALBUMS)} album covers...")
    success = 0
    for artist, album, dest in ALBUMS:
        if os.path.exists(dest):
            print(f"Exists: {dest}")
            success += 1
            continue
        if scrape_cover(artist, album, dest):
            success += 1
    print(f"Done! {success}/{len(ALBUMS)} covers ready.")

if __name__ == '__main__':
    main()
