// Development-only fixtures. Production builds remove this module's import branch.
import { defaultPreferences, emptyPlayback, emptyTask } from "./bridge";
import type {
  Album,
  Artist,
  Library,
  Playback,
  Preferences,
  Track,
} from "./types";
const fixtureAssets = Object.fromEntries(
  Object.entries(
    import.meta.glob("../../assets/{covers,artists}/*.png", {
      eager: true,
      query: "?url",
      import: "default",
    }),
  ).map(([p, u]) => [p.replace("../../", ""), u as string]),
);
const seeds = [
  ["Plastic Beach", "Gorillaz", "Beach"],
  ["Demon Days", "Gorillaz", "DemonDays"],
  ["Random Access Memories", "Daft Punk", "RAM"],
  ["Discovery", "Daft Punk", "Discovery"],
  ["Currents", "Tame Impala", "Currents"],
  ["The Slow Rush", "Tame Impala", "SlowRush"],
  ["Hurry Up, We’re Dreaming", "M83", "Clouds"],
  ["Fantasy", "M83", "NightVisions"],
  ["Junk", "M83", "Pier"],
  ["AM", "Arctic Monkeys", "Overcast"],
  ["Parachutes", "Coldplay", "Lake"],
  ["Channel Orange", "Frank Ocean", "SaltyRoads"],
];
const tracks: Track[] = [];
const albums: Album[] = [];
const artists: Artist[] = [];
for (let a = 0; a < seeds.length; a++) {
  const [title, name, cover] = seeds[a];
  let artist = artists.find((artist) => artist.name === name);
  if (!artist) {
    artist = {
      id: `local/artist/${artists.length}`,
      name,
      portrait: name === "M83" ? fixtureAssets["assets/artists/m83.png"] : null,
      album_ids: [],
      track_ids: [],
    };
    artists.push(artist);
  }
  const album: Album = {
    id: `local/album/${a}`,
    artist_id: artist.id,
    title,
    artist: name,
    cover: fixtureAssets[`assets/covers/${cover}.png`],
    genre: ["Electronic", "Alternative", "Rock"][a % 3],
    year: 2011,
    track_ids: [],
    duration: 0,
  };
  artist.album_ids.push(album.id);
  for (let n = 0; n < 12; n++) {
    const id = tracks.length + 1;
    const song =
      a === 6
        ? [
            "Intro",
            "Midnight City",
            "Reunion",
            "Where the Boats Go",
            "Wait",
            "Raconte-Moi Une Histoire",
          ][n % 6]
        : [
            "Opening",
            "Second Light",
            "A Long Way Home Through the City at Night",
            "Interlude",
            "Afterglow",
            "Closing",
          ][n % 6];
    tracks.push({
      id,
      path: `/fixture/${id}.flac`,
      title: song,
      artist: name,
      album: title,
      track_number: n + 1,
      disc_number: 1,
      duration: 157 + n * 13,
      genre: album.genre,
      year: album.year,
      stable_id: `fixture-${id}`,
      favorited: n === 1,
      available: true,
      audio_quality:{format:"FLAC",lossless:true,sample_rate:44100,bit_depth:16,bitrate:900,channels:2},
      cover_art_path: album.cover,
    });
    album.track_ids.push(id);
    artist.track_ids.push(id);
    album.duration += 157 + n * 13;
  }
  albums.push(album);
}
if (new URLSearchParams(location.search).has("largeAlbums")) {
  for (let index = albums.length; index < 1000; index++) {
    const source = albums[index % seeds.length];
    const track = { ...tracks.find(t => t.id === source.track_ids[0])!, id: tracks.length + 1, stable_id: `fixture-grid-${index}` };
    const album = { ...source, id: `local/album/${index}`, title: `Album ${String(index).padStart(4, "0")} with a long title`, track_ids: [track.id], duration: track.duration };
    track.album = album.title;
    tracks.push(track);
    albums.push(album);
    const artist = artists.find(a => a.id === album.artist_id)!;
    artist.album_ids.push(album.id);
    artist.track_ids.push(track.id);
  }
}
if (new URLSearchParams(location.search).has("large"))
  for (let i = tracks.length; i < 10000; i++)
    tracks.push({
      ...tracks[i % 144],
      id: i + 1,
      stable_id: `fixture-${i + 1}`,
      title: `Library song ${i + 1}`,
    });
if (new URLSearchParams(location.search).has('contributors')) {
 const album=albums.find(a=>a.title==='Junk')!;
 for(let i=0;i<17;i++)artists.push({id:`local/artist/guest-${i}`,name:i===0?'A contributor with a particularly long name':`Guest artist ${i+1}`,portrait:i%2?fixtureAssets['assets/artists/m83.png']:null,album_ids:[album.id],track_ids:[album.track_ids[i%album.track_ids.length]]});
}
if(new URLSearchParams(location.search).has("consistency")){albums[0].release_type="single";albums[0].release_type_source="file_tag";tracks[1].cover_art_path=albums[1].cover;}
const library: Library = {
  tracks,
  albums,
  artists,
  genres: ["Electronic", "Alternative", "Rock"].map((name) => ({
    id: `genre/${name}`,
    name,
    track_ids: tracks.filter((t) => t.genre === name).map((t) => t.id),
    album_ids: albums.filter((a) => a.genre === name).map((a) => a.id),
  })),
  sources: [
    {
      id: 1,
      display_name: "Test library",
      kind: "reference",
      enabled: true,
      path: "/fixture",
      available: true,
      last_error: null,
    },
  ],
};
let integrationStatus = {
  config: { discord_enabled: false, discord_client_id: "", lastfm_enabled: false, lastfm_username: null },
  discord: "Off", lastfm: "Off", queued: 0,
};
let player: Playback = { ...emptyPlayback };
let preferences: Preferences =
  JSON.parse(localStorage.getItem("musicos-fixture-preferences") ?? "null") ??
  defaultPreferences;
let savedCollections: import("./types").SavedCollection[]=JSON.parse(localStorage.getItem("musicos-fixture-collections")??"[]");
let metadataSuggestions=tracks.slice(0,3).map(t=>({track_id:t.id,current_title:t.title,current_artist:t.artist,current_album:t.album,current_year:t.year,current_artwork:t.cover_art_path,revision:1,candidate_token:String(t.id),candidate:{title:t.title,artist:t.artist,album:`${t.album} (Deluxe)`,year:2012,confidence:.82,recording_mbid:'example-recording',release_mbid:'example-release',artwork_path:t.cover_art_path}}));
let metadataHistory:import('./types').MetadataHistory[]=[];
export async function fixtureCall(
  command: string,
  args: unknown,
): Promise<unknown> {
  const a = (args ?? {}) as Record<string, any>;
  switch (command) {
    case 'metadata_status': return {cached:8,checking:a.retry?2:0,unavailable:2,retryable:a.retry?0:2};
    case 'sparklist': return {ready:false,reason:'The music model is awaiting listening evaluation and a reviewed release download. Audio generation is unavailable.',download_bytes:277183168,analyzed:0,available:tracks.filter(t=>t.available).length,analyzed_ids:[],preview:null,items:savedCollections,chosen_id:null};
    case 'depth_setup': return {supported:true,ready:true,running:false,downloaded:99188950,total:99188950,error:null,revision:'fixture',attribution:'Depth renderer fixture'};
    case 'depth_artwork': return {fresh:new URLSearchParams(location.search).has('freshDepth'),photo:a.path,map:'data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256"><defs><radialGradient id="g"><stop stop-color="white"/><stop offset="1" stop-color="black"/></radialGradient></defs><rect width="256" height="256" fill="url(#g)"/></svg>')};
    case 'artwork_performance': return;
    case 'audio_analysis': return {running:false,paused:false,processed:0,total:0,features:[],errors:[]};
    case 'audio_specs': return {format:'FLAC',sample_rate:44100,bit_depth:16,bitrate:900,channels:2,lossless:true};
    case 'library_source': {const source=library.sources.find(s=>s.id===a.id);if(!source)throw new Error('Folder no longer exists');source.enabled=a.enabled;return;}
    case "toggle_favorite": {const track=tracks.find(t=>t.id===a.trackId);if(!track)throw new Error('Song no longer exists');track.favorited=!track.favorited;return track.favorited;}
    case "collections": {
      const r=a.request;
      if(r){
        if(r.action==='create'||r.action==='create_spark')savedCollections.push({id:`${r.action==='create'?'playlist':'spark'}/${Date.now()}`,name:(r.name??r.definition.name).trim(),track_ids:[],spark:r.definition??null,kind:r.action==='create'?'manual':'rule',description:'',artwork:null,artwork_choice:r.artwork_choice??'playlist/tritower'});
        else {const item=savedCollections.find(c=>c.id===r.id);if(!item)throw new Error('Collection no longer exists');if(r.action==='delete')savedCollections=savedCollections.filter(c=>c.id!==r.id);if(r.action==='rename')item.name=r.name.trim();if(r.action==='describe')item.description=r.description;if(r.action==='cover')item.artwork_choice=r.artwork_choice;if(r.action==='update_spark'){item.spark=r.definition;item.name=r.definition.name;}if(r.action==='add')item.track_ids=[...new Set([...item.track_ids,...r.track_ids])];if(r.action==='remove')item.track_ids=item.track_ids.filter(id=>!r.track_ids.includes(id));if(r.action==='reorder')item.track_ids=r.track_ids;}
        localStorage.setItem('musicos-fixture-collections',JSON.stringify(savedCollections));
      }
      return savedCollections.map(c=>c.spark?{...c,track_ids:tracks.filter(t=>c.spark!.rules.every(rule=>rule.field==='favorited'?t.favorited:String(t[rule.field as keyof Track]).toLowerCase().includes(rule.value.toLowerCase()))).map(t=>t.id)}:{...c});
    }

    case "integration_status": return integrationStatus;
    case "integration_action":
      if (a.request.action !== "configure") throw new Error("Open MusicOS desktop to connect your account.");
      if (a.request.discord_enabled && !/^\d{17,20}$/.test(a.request.discord_client_id)) throw new Error("Enter the Application ID from your Discord developer application.");
      if (a.request.lastfm_enabled) throw new Error("Connect Last.fm before enabling scrobbling.");
      integrationStatus = { ...integrationStatus, config: { ...integrationStatus.config, ...a.request }, discord: a.request.discord_enabled ? "Browser preview · open MusicOS desktop to connect" : "Off" };
      return { status: integrationStatus, auth_url: null };
    case "bootstrap":
      return { library, preferences, playback: player, task: emptyTask };
    case "refresh_library":
      return library;
    case "save_preferences":
      preferences = a.preferences;
      localStorage.setItem(
        "musicos-fixture-preferences",
        JSON.stringify(preferences),
      );
      return preferences;
    case "album_info":return new URLSearchParams(location.search).has('contributors')?{description:'A collection of songs featuring musicians from across the world. '.repeat(20),source_url:'https://en.wikipedia.org/wiki/Junk_(M83_album)'}:{description:null,source_url:null};
    case "artist_info":
      return {
        lookup_status: new URLSearchParams(location.search).get('profiles')==='pending'?'loading':'cached',
        biography:
          "M83 is a French electronic music project formed in Antibes in 1999. Initially a duo of Nicolas Fromageau and Anthony Gonzalez, it is now led by Gonzalez. The project combines electronic music, ambient textures, and cinematic arrangements. Their records include Hurry Up, We’re Dreaming and Junk.",
        portrait:
          a.name === "M83" ? fixtureAssets["assets/artists/m83.png"] : null,
        source_url: "https://en.wikipedia.org/wiki/M83_(band)",
        artist_mbid: "fixture",
      };
    case "artwork_palette":
      return {
        background: "#c8ded9",
        accent: "#20574d",
        blob: "#55937a",
        text: "#12202a",
      };
    case "playback": {
      const action = a.action;
      if (action.type === "play") {
        player = {
          ...player,
          queue: action.ids.map((id: number) =>
            tracks.find((t) => t.id === id),
          ),
          queue_entry_ids:action.ids.map((_:number,i:number)=>i+1),
          queue_index: action.start_index,
          is_playing: true,
          position: 0,
        };
        player.track = player.queue[action.start_index];
      }
      if(action.type==='play_next'||action.type==='add_to_queue') {
        const song=tracks.find(t=>t.id===action.id);
        if(song){const queue=[...player.queue],keys=[...(player.queue_entry_ids??queue.map((_,i)=>i+1))],at=action.type==='play_next'?(player.queue_index??-1)+1:Math.max((player.queue_index??-1)+1,Math.min(queue.length,action.index??queue.length));queue.splice(at,0,song);keys.splice(at,0,Math.max(0,...keys)+1);player={...player,queue,queue_entry_ids:keys};}
      }
      if (action.type === "toggle")
        player = { ...player, is_playing: !player.is_playing };
      if (action.type === "next" || action.type === "previous") {
        const index =
          ((player.queue_index ?? 0) +
            (action.type === "next" ? 1 : -1) +
            player.queue.length) %
          player.queue.length;
        player = { ...player, queue_index: index, track: player.queue[index] };
      }
      if (action.type === "seek")
        player = { ...player, position: action.position };
      if (action.type === "volume")
        player = { ...player, volume: action.volume };
      if (action.type === "shuffle")
        player = { ...player, shuffle: !player.shuffle };
      if (action.type === "repeat")
        player = {
          ...player,
          repeat:
            player.repeat === "none"
              ? "all"
              : player.repeat === "all"
                ? "one"
                : "none",
        };
      if (action.type === "select")
        player = {
          ...player,
          queue_index: action.index,
          track: player.queue[action.index],
        };
      if(action.type==='remove'&&action.index>=0&&action.index<player.queue.length) {
        const queue=player.queue.filter((_,index)=>index!==action.index),entries=player.queue_entry_ids?.filter((_,index)=>index!==action.index);
        let index=player.queue_index;
        if(index!==null){if(action.index<index)index--;else if(action.index===index)index=queue.length?Math.min(index,queue.length-1):null;}
        player={...player,queue,queue_entry_ids:entries,queue_index:index,track:index===null?null:queue[index],is_playing:queue.length>0&&player.is_playing};
      }
      if(action.type==='move') {const entries=[...(player.queue_entry_ids??player.queue.map((_,i)=>i+1))];const [key]=entries.splice(action.from,1);entries.splice(action.to,0,key);player={...player,queue_entry_ids:entries};const q=[...player.queue],current=player.queue_index;const [song]=q.splice(action.from,1);if(song)q.splice(action.to,0,song);let index=current;if(index!==null){if(index===action.from)index=action.to;else if(action.from<index&&action.to>=index)index--;else if(action.from>index&&action.to<=index)index++;}player={...player,queue:q,queue_index:index};}
      if(action.type==='clear_upcoming')player={...player,queue:player.queue.slice(0,(player.queue_index??-1)+1),queue_entry_ids:player.queue_entry_ids?.slice(0,(player.queue_index??-1)+1)};
      if(action.type==='dj_mode')player={...player,dj_mode:action.enabled,shuffle:action.enabled?false:player.shuffle};
      return player;
    }
    case "metadata_edit": {
      const changes=library.tracks.filter(t=>a.ids.includes(t.id)).map(t=>({track_id:t.id,before:{...t},after:{...t,...a.patch},current:null,conflict:false}));
      for(const track of library.tracks)if(a.ids.includes(track.id))Object.assign(track,a.patch);
      metadataHistory.unshift({id:Date.now(),created_at:Date.now()/1000,undone:false,conflict:false,changes});return a.ids.length;
    }
    case 'metadata_workspace':{
      const r=a.request;
      if(r.action==='history')return {history:metadataHistory};
      if(r.action==='undo'){const h=metadataHistory.find(h=>h.id===r.id);if(h){for(const c of h.changes){const t=tracks.find(t=>t.id===c.track_id);if(t)Object.assign(t,c.before);}h.undone=true;}return {history:metadataHistory};}
      if(r.action==='apply'||r.action==='dismiss'){const s=metadataSuggestions.find(s=>s.track_id===r.trackId);if(!s||s.candidate_token!==r.token||s.revision!==r.revision)throw new Error('This suggestion changed. Refresh the comparison.');
       if(r.action==='apply'){const t=tracks.find(t=>t.id===s.track_id)!;const before={...t};for(const field of r.fields??[])Object.assign(t,{[field==='artwork'?'cover_art_path':field]:s.candidate[field==='artwork'?'artwork_path':field as keyof typeof s.candidate]});metadataHistory.unshift({id:Date.now(),created_at:Date.now()/1000,undone:false,conflict:false,changes:[{track_id:t.id,before,after:{...t},current:null,conflict:false}]});}
       metadataSuggestions=metadataSuggestions.filter(s=>s.track_id!==r.trackId);
      }
      const filtered=metadataSuggestions.filter(s=>`${s.current_title} ${s.current_artist} ${s.current_album}`.toLowerCase().includes((r.search??'').toLowerCase()));return {items:filtered.slice(r.offset??0,(r.offset??0)+50),total:filtered.length,offset:r.offset??0};
    }
    case "metadata_review":
      return [];
    case "cancel_search":
    case "cancel_task":
      return;
    case "device":
      return {
        target: "http://127.0.0.1:18080",
        connected: false,
        error: "No test device connected.",
        preview: { running: false },
      };
    case "discover":
      return [
        {
          id: "apple/album/100",
          kind: "album",
          title: "Online album",
          artist: "Online artist",
          artist_id: "apple/artist/200",
          album_id: "apple/album/100",
          album: "Online album",
          cover: null,
          genre: "Electronic",
          duration: null,
          track_number: null,
          store_url: "https://music.apple.com/us/album/100",
          local: false,
        },
      ];
    default:
      throw Error(`Fixture command not supported: ${command}`);
  }
}
