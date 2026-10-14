import { describe, it, expect } from "vitest";
import {
  initialNavigation,
  navigationReducer,
  restoreNavigation,
  themeSource,
} from "../src/navigation";
describe("library navigation", () => {
  it("restores search, scroll and selection through album and artist routes", () => {
    const grid = {
      ...initialNavigation,
      current: {
        view: "albums" as const,
        search: "dream",
        scroll: 422,
        selected: "album-7",
      },
    };
    const album = navigationReducer(grid, {
      type: "open",
      route: { view: "album", id: "album-7", scroll: 110, search: "" },
    });
    const artist = navigationReducer(album, {
      type: "open",
      route: { view: "artist", id: "artist-2", scroll: 0, search: "" },
    });
    expect(navigationReducer(artist, { type: "back" }).current).toEqual(
      album.current,
    );
    expect(
      navigationReducer(navigationReducer(artist, { type: "back" }), {
        type: "back",
      }).current,
    ).toEqual(grid.current);
  });
  it("remembers each library scope when switching between collections", () => {
    const songs = {
      ...initialNavigation,
      current: {
        view: "songs" as const,
        scroll: 5000,
        search: "instrumental",
        selected: "17",
      },
    };
    const albums = navigationReducer(songs, { type: "scope", view: "albums" });
    const returned = navigationReducer(albums, {
      type: "scope",
      view: "songs",
    });
    expect(returned.current).toEqual(songs.current);
  });
  it("rejects stale persisted routes", () => {
    expect(restoreNavigation("{broken")).toEqual(initialNavigation);
    expect(
      restoreNavigation('{"current":{"view":"unknown"},"history":[]}'),
    ).toEqual(initialNavigation);
  });
});
it("does not push the same detail onto its own back history",()=>{
 const state=navigationReducer(initialNavigation,{type:"open",route:{view:"artist",id:"one",scroll:94,search:""}});
 expect(navigationReducer(state,{type:"open",route:{view:"artist",id:"one",scroll:0,search:""}})).toEqual(state);
 expect(restoreNavigation(JSON.stringify({current:{view:"discover"},history:[]}))).toEqual(initialNavigation);
});
describe("artwork source precedence", () => {
  it("uses the viewed album", () =>
    expect(
      themeSource("album", "album", "portrait", "fallback", "playing"),
    ).toBe("album"));
  it("uses portrait then artist album", () => {
    expect(
      themeSource("artist", "album", "portrait", "fallback", "playing"),
    ).toBe("portrait");
    expect(themeSource("artist", null, null, "fallback", "playing")).toBe(
      "fallback",
    );
  });
  it("keeps missing detail art blue and uses playback elsewhere", () => {
    expect(themeSource("album", null, null, null, "playing")).toBeNull();
    expect(themeSource("songs", null, null, null, "playing")).toBe("playing");
  });
});

it("skips collections removed by a metadata regroup while retaining the browsing location",()=>{
 const grid={...initialNavigation,current:{view:"artists" as const,scroll:200,search:"test"}};
 const detail=navigationReducer(grid,{type:"open",route:{view:"artist",id:"old",scroll:0,search:""}});
 const album=navigationReducer(detail,{type:"open",route:{view:"album",id:"record",scroll:0,search:""}});
 const changed=navigationReducer(album,{type:"reconcile",albums:["record"],artists:["new"],genres:[]});
 expect(navigationReducer(changed,{type:"back"}).current).toEqual(grid.current);
 expect(navigationReducer(detail,{type:"reconcile",albums:[],artists:["new"],genres:[]}).current).toEqual(grid.current);
});
