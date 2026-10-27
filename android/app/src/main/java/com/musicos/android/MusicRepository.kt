package com.musicos.android

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.json.JSONArray

/** Offline files are immutable, checksum-named objects; library replacement is transactional. */
class MusicRepository(private val context: Context) :
    SQLiteOpenHelper(context, "musicos-offline-v1.db", null, 1) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val library = MutableStateFlow(MusicLibrary())
    val snapshots = library.asStateFlow()

    init {
        scope.launch { refresh() }
    }

    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL(
            "CREATE TABLE songs(id TEXT PRIMARY KEY,server_id TEXT,title TEXT,artist TEXT,album TEXT,album_id TEXT,genre TEXT,year INTEGER,track_no INTEGER,disc_no INTEGER,duration INTEGER,size INTEGER,sha TEXT,mime TEXT,art_sha TEXT,local_file TEXT,art_file TEXT,favorite INTEGER,favorite_dirty INTEGER DEFAULT 0,play_count INTEGER DEFAULT 0,last_played INTEGER DEFAULT 0)"
        )
        db.execSQL(
            "CREATE TABLE collections(id TEXT PRIMARY KEY,server_id TEXT,name TEXT,kind TEXT,sequence TEXT)"
        )
        db.execSQL("CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT)")
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        error("Unsupported library version; offline music was retained")
    }

    fun song(id: String) = library.value.songs.find { it.id == id }

    fun objectFile(hash: String) =
        File(context.filesDir, "music/${hash.requireHash()}.audio").also {
            it.parentFile!!.mkdirs()
        }

    fun artworkFile(hash: String) =
        File(context.filesDir, "artwork/${hash.requireHash()}.jpg").also {
            it.parentFile!!.mkdirs()
        }

    @Synchronized
    fun refresh() {
        val tracks = mutableListOf<Song>()
        val collections = mutableListOf<MusicCollection>()
        val db = readableDatabase
        db.rawQuery(
                "SELECT id,server_id,title,artist,album,album_id,genre,year,track_no,disc_no,duration,size,sha,mime,art_sha,local_file,art_file,favorite FROM songs ORDER BY album COLLATE NOCASE,disc_no,track_no,title COLLATE NOCASE",
                null,
            )
            .use { c ->
                while (c.moveToNext()) tracks +=
                    Song(
                        c.getString(0),
                        c.getString(1),
                        c.getString(0).substringAfterLast('/').toLong(),
                        c.getString(2),
                        c.getString(3),
                        c.getString(4),
                        c.getString(5),
                        c.getString(6),
                        c.getInt(7),
                        c.getInt(8),
                        c.getInt(9),
                        c.getLong(10),
                        c.getLong(11),
                        c.getString(12),
                        c.getString(13),
                        c.getString(14),
                        c.getString(17).toInt() == 1,
                        c.getString(15)?.takeIf { File(it).isFile },
                        c.getString(16)?.takeIf { File(it).isFile },
                    )
            }
        db.rawQuery(
                "SELECT id,server_id,name,kind,sequence FROM collections ORDER BY name COLLATE NOCASE",
                null,
            )
            .use { c ->
                while (c.moveToNext()) {
                    val ids = JSONArray(c.getString(4))
                    collections +=
                        MusicCollection(
                            c.getString(0),
                            c.getString(1),
                            c.getString(2),
                            c.getString(3),
                            (0 until ids.length()).map { ids.getString(it) },
                        )
                }
            }
        val name =
            db.rawQuery("SELECT value FROM meta WHERE key='display_name'", null).use {
                if (it.moveToFirst()) it.getString(0) else ""
            }
        library.value = MusicLibrary(tracks, collections, name, true)
    }

    @Synchronized
    fun import(manifest: SyncManifest, files: Map<String, File>, artwork: Map<String, File>) {
        require(manifest.tracks.all { files[it.id]?.isFile == true }) {
            "Finish verifying the download before importing"
        }
        val db = writableDatabase
        db.beginTransaction()
        try {
            for (song in manifest.tracks) {
                val favorite =
                    db.rawQuery(
                            "SELECT favorite,favorite_dirty FROM songs WHERE id=?",
                            arrayOf(song.id),
                        )
                        .use {
                            if (it.moveToFirst() && it.getInt(1) == 1) it.getInt(0)
                            else if (song.favorite) 1 else 0
                        }
                val values =
                    ContentValues().apply {
                        put("id", song.id)
                        put("server_id", song.serverId)
                        put("title", song.title)
                        put("artist", song.artist)
                        put("album", song.album)
                        put("album_id", song.albumId)
                        put("genre", song.genre)
                        put("year", song.year)
                        put("track_no", song.trackNumber)
                        put("disc_no", song.discNumber)
                        put("duration", song.duration)
                        put("size", song.size)
                        put("sha", song.hash)
                        put("mime", song.mime)
                        put("art_sha", song.artwork)
                        put("local_file", files.getValue(song.id).path)
                        put("art_file", song.artwork?.let { artwork[it]?.path })
                        put("favorite", favorite)
                    }
                if (db.update("songs", values, "id=?", arrayOf(song.id)) == 0)
                    db.insertOrThrow("songs", null, values)
            }
            for (collection in manifest.collections) db.insertWithOnConflict(
                "collections",
                null,
                ContentValues().apply {
                    put("id", collection.id)
                    put("server_id", collection.serverId)
                    put("name", collection.name)
                    put("kind", collection.kind)
                    put("sequence", JSONArray(collection.songIds).toString())
                },
                SQLiteDatabase.CONFLICT_REPLACE,
            )
            if (manifest.displayName.isNotBlank())
                db.insertWithOnConflict(
                    "meta",
                    null,
                    ContentValues().apply {
                        put("key", "display_name")
                        put("value", manifest.displayName)
                    },
                    SQLiteDatabase.CONFLICT_REPLACE,
                )
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
        refresh()
    }

    fun favorite(id: String) {
        scope.launch {
            writableDatabase.execSQL(
                "UPDATE songs SET favorite=1-favorite,favorite_dirty=1 WHERE id=?",
                arrayOf(id),
            )
            refresh()
        }
    }

    fun played(id: String) {
        scope.launch {
            writableDatabase.execSQL(
                "UPDATE songs SET play_count=play_count+1,last_played=? WHERE id=?",
                arrayOf<Any>(System.currentTimeMillis(), id),
            )
        }
    }

    fun recents(): List<Song> {
        val ids =
            readableDatabase
                .rawQuery(
                    "SELECT id FROM songs WHERE last_played>0 ORDER BY last_played DESC LIMIT 20",
                    null,
                )
                .use { c -> buildList { while (c.moveToNext()) add(c.getString(0)) } }
        return ids.mapNotNull(::song)
    }
}
