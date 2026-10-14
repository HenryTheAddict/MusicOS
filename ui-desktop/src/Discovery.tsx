import { useEffect, useState, type RefObject } from "react";
import { call, errorText } from "./bridge";
import {
  CollectionGrid,
  Cover,
  Empty,
  Icon,
  collectionDuration,
  duration,
} from "./components";
import type { CatalogItem, Route, Scope } from "./types";
const genres = [
  "Alternative",
  "Electronic",
  "Rock",
  "Pop",
  "Hip-Hop/Rap",
  "R&B/Soul",
  "Jazz",
  "Classical",
  "Country",
  "Dance",
];
export default function Discovery({
  route,
  onOpen,
  onChange,
  scrollRef,
  scale,
  onError,
}: {
  route: Route;
  onOpen: (route: Route) => void;
  onChange: (patch: Partial<Route>) => void;
  scrollRef: RefObject<HTMLElement | null>;
  scale: number;
  onError: (error: string) => void;
}) {
  const kind = (
    route.view === "discover" &&
    ["albums", "artists", "genres", "songs"].includes(route.id ?? "")
      ? route.id
      : "albums"
  ) as Scope;
  const term = route.search;
  const [results, setResults] = useState<CatalogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const item = route.catalog;
  useEffect(() => {
    let gone = false;
    setResults([]);
    setError("");
    const lookup = route.view === "catalog" ? item?.id : undefined;
    const requestedKind =
      route.view === "catalog"
        ? item?.kind === "artist"
          ? "albums"
          : "songs"
        : kind;
    if (!lookup && term.trim().length < 2) {
      setLoading(false);
      return;
    }
    setLoading(true);
    const timer = setTimeout(
      () => {
        void call("discover", {
          term: lookup ? "" : term.trim(),
          kind: requestedKind,
          lookup,
        })
          .then((rows) => {
            if (!gone)
              setResults(rows.filter((r) => !lookup || r.id !== lookup));
          })
          .catch((e) => {
            if (!gone) setError(errorText(e));
          })
          .finally(() => {
            if (!gone) setLoading(false);
          });
      },
      lookup ? 0 : 450,
    );
    return () => {
      gone = true;
      clearTimeout(timer);
      void call("cancel_search").catch(() => {});
    };
  }, [term, kind, route.view, item?.id]);
  function openItem(item: CatalogItem) {
    if (item.kind === "song") {
      if (item.album_id)
        onOpen({
          view: "catalog",
          id: item.album_id,
          search: term,
          scroll: 0,
          catalog: {
            ...item,
            id: item.album_id,
            kind: "album",
            title: item.album ?? item.title,
          },
        });
      else if (item.store_url)
        void call("open_provider", { url: item.store_url }).catch((e) =>
          onError(errorText(e)),
        );
      return;
    }
    onOpen({
      view: "catalog",
      id: item.id,
      search: term,
      scroll: 0,
      catalog: item,
    });
  }
  function provider(item: CatalogItem) {
    if (item.store_url)
      void call("open_provider", { url: item.store_url }).catch((e) =>
        onError(errorText(e)),
      );
  }
  const cards = (rows: CatalogItem[]) => (
    <CollectionGrid
      items={rows.map((item) => ({
        id: item.id,
        title: item.title,
        subtitle: item.kind === "artist" ? "Artist" : item.artist,
        cover: item.cover,
        portrait: item.kind === "artist",
        onClick: () => openItem(item),
        menu: item.store_url
          ? [
              {
                label: "Open in Apple Music",
                icon: "NetSearch",
                onSelect: () => provider(item),
              },
            ]
          : undefined,
      }))}
      scrollRef={scrollRef}
      scale={scale}
      initialOffset={route.scroll}
      onSelect={() => {}}
    />
  );
  return (
    <section className="discovery">
      {route.view === "catalog" && item ? (
        <>
          <header className="album-hero catalog-hero">
            <Cover
              src={item.cover}
              title={item.title}
              portrait={item.kind === "artist"}
            />
            <div className="album-copy">
              <span className="eyebrow">Online discovery</span>
              <h1>{item.title}</h1>
              <button
                className="artist-link"
                disabled={item.kind === "artist" || !item.artist_id}
                onClick={() =>
                  item.artist_id &&
                  onOpen({
                    view: "catalog",
                    id: item.artist_id,
                    search: term,
                    scroll: 0,
                    catalog: {
                      ...item,
                      id: item.artist_id,
                      kind: "artist",
                      title: item.artist,
                      cover: null,
                    },
                  })
                }
              >
                {item.artist}
              </button>
              <p>
                {item.genre}
                {item.duration ? ` · ${collectionDuration(item.duration)}` : ""}
              </p>
              <p className="muted">Not in your local library.</p>
              {item.store_url && (
                <button className="primary" onClick={() => provider(item)}>
                  Open in Apple Music ↗
                </button>
              )}
            </div>
          </header>
          {item.kind === "artist" ? (
            <>
              <h2 className="section-title">Albums</h2>
              {cards(results.filter((r) => r.kind === "album"))}
            </>
          ) : (
            <div className="online-tracks">
              {results
                .filter((r) => r.kind === "song")
                .map((song, i) => (
                  <article className="track-row" key={song.id}>
                    <span>{song.track_number ?? i + 1}</span>
                    <div className="track-title">
                      <div>
                        <strong>{song.title}</strong>
                        <small>{song.artist}</small>
                      </div>
                    </div>
                    <span className="online-label">Online only</span>
                    <span>{duration(song.duration ?? 0)}</span>
                    <button
                      className="provider-link"
                      disabled={!song.store_url}
                      aria-label={`Open ${song.title} in Apple Music`}
                      onClick={() => provider(song)}
                    >
                      ↗
                    </button>
                  </article>
                ))}
            </div>
          )}
        </>
      ) : (
        <>
          <header className="page-heading">
            <span className="eyebrow">Beyond your library</span>
            <h1>Find your next favorite.</h1>
            <p>Explore music online. Your local library stays yours.</p>
          </header>
          <label className="discovery-search">
            <Icon name="search" />
            <input
              autoFocus
              aria-label="Search online music"
              placeholder="Albums, artists, songs…"
              value={term}
              onChange={(e) => onChange({ search: e.target.value, scroll: 0 })}
            />
          </label>
          <nav className="discovery-scopes" aria-label="Discovery categories">
            {(["albums", "artists", "genres", "songs"] as Scope[]).map(
              (scope) => (
                <button
                  key={scope}
                  aria-pressed={scope === kind}
                  onClick={() => onChange({ id: scope, scroll: 0 })}
                >
                  {scope[0].toUpperCase() + scope.slice(1)}
                </button>
              ),
            )}
          </nav>
          {kind === "genres" && (
            <div className="genre-chips">
              {genres.map((g) => (
                <button
                  key={g}
                  onClick={() => onChange({ search: g, scroll: 0 })}
                >
                  {g}
                </button>
              ))}
            </div>
          )}
          {!loading && !error && results.length > 0 && cards(results)}
          {!loading &&
            !error &&
            results.length === 0 &&
            term.trim().length >= 2 && (
              <Empty title="No matches found.">
                Try another album, artist, or song.
              </Empty>
            )}
          {term.trim().length < 2 && (
            <Empty title="A whole world of music.">
              Search above to explore albums, artists, and songs. Online results
              link to Apple Music.
            </Empty>
          )}
        </>
      )}
      {loading && (
        <div className="loading" role="status">
          Searching the catalog…
        </div>
      )}
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
      <p className="provider-credit">
        Catalog and artwork from Apple. Online discoveries are for browsing;
        playback uses your local files.
      </p>
    </section>
  );
}
