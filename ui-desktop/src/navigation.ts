import type { Route } from "./types";
export interface Navigation {
  current: Route;
  history: Route[];
  locations?: Partial<Record<Route["view"], Route>>;
}
export const initialNavigation: Navigation = {
  current: { view: "home", search: "", scroll: 0 },
  history: [],
};
export type NavigationAction =
  | { type: "open"; route: Route }
  | { type:"reconcile"; albums:string[]; artists:string[]; genres:string[] }
  | { type: "back" }
  | { type: "patch"; patch: Partial<Route> }
  | { type: "scope"; view: Route["view"] };
export function navigationReducer(
  state: Navigation,
  action: NavigationAction,
): Navigation {
  const locations = { ...state.locations };
  if (!["album", "artist", "genre", "catalog", "playlist"].includes(state.current.view))
    locations[state.current.view] = state.current;
  switch (action.type) {
    case "reconcile": {
      const valid=(route:Route)=>route.view==='album'?action.albums.includes(route.id??''):route.view==='artist'?action.artists.includes(route.id??''):route.view==='genre'?action.genres.includes(route.id??''):true;
      const history=state.history.filter(valid);
      if(valid(state.current))return history.length===state.history.length?state:{...state,history};
      return {...state,current:history.at(-1)??initialNavigation.current,history:history.slice(0,-1)};
    }
    case "open":
      if(state.current.view===action.route.view && state.current.id===action.route.id)return state;
      return {
        locations,
        current: action.route,
        history: [...state.history, state.current],
      };
    case "back":
      return state.history.length
        ? {
            ...state,
            current: state.history.at(-1)!,
            history: state.history.slice(0, -1),
          }
        : state;
    case "patch":
      return { ...state, current: { ...state.current, ...action.patch } };
    case "scope":
      return {
        locations,
        current: locations[action.view] ?? {
          view: action.view,
          search: "",
          scroll: 0,
        },
        history: [],
      };
  }
}
export function restoreNavigation(raw: string | null): Navigation {
  try {
    const state = JSON.parse(raw ?? "null");
    const allowed = [
      "home",
      "search",
      "albums",
      "artists",
      "genres",
      "songs",
      "album",
      "artist",
      "genre",
      "favorites",
      "playlist",
      "playlists",
      "sparklists",
      "sources",
      "review",
    ];
    if (
      state &&
      allowed.includes(state.current?.view) &&
      Array.isArray(state.history) &&
      state.history.every((r: Route) => allowed.includes(r.view))
    )
      return state;
  } catch {}
  return initialNavigation;
}
export function themeSource(
  view: Route["view"],
  albumCover?: string | null,
  artistPortrait?: string | null,
  artistAlbumCover?: string | null,
  playingCover?: string | null,
) {
  if (view === "album") return albumCover ?? null;
  if (view === "artist") return artistPortrait ?? artistAlbumCover ?? null;
  return playingCover ?? null;
}
