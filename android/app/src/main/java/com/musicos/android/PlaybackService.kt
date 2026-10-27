package com.musicos.android

import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.HeartRating
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.Rating
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.LibraryResult
import androidx.media3.session.MediaLibraryService
import androidx.media3.session.MediaSession
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.google.common.collect.ImmutableList
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/** One player survives Activity destruction, lock-screen use and controller reconnects. */
@androidx.annotation.OptIn(UnstableApi::class)
class PlaybackService : MediaLibraryService() {
    private lateinit var player: ExoPlayer
    private var session: MediaLibrarySession? = null
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val handler = Handler(Looper.getMainLooper())
    private val repository
        get() = (application as MusicOSApp).repository

    private val saved by lazy { getSharedPreferences("playback-v1", MODE_PRIVATE) }
    private val savePosition =
        object : Runnable {
            override fun run() {
                persist(false)
                if (player.isPlaying) handler.postDelayed(this, 2000)
            }
        }

    override fun onCreate() {
        super.onCreate()
        player =
            ExoPlayer.Builder(this)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
                        .build(),
                    true,
                )
                .setHandleAudioBecomingNoisy(true)
                .setWakeMode(C.WAKE_MODE_LOCAL)
                .build()
        player.addListener(
            object : Player.Listener {
                override fun onEvents(p: Player, events: Player.Events) {
                    if (
                        events.contains(Player.EVENT_TIMELINE_CHANGED) ||
                            events.contains(Player.EVENT_MEDIA_ITEM_TRANSITION)
                    )
                        persist(true)
                    if (events.contains(Player.EVENT_IS_PLAYING_CHANGED)) {
                        handler.removeCallbacks(savePosition)
                        if (p.isPlaying) handler.post(savePosition) else persist(false)
                    }
                    if (
                        events.contains(Player.EVENT_SHUFFLE_MODE_ENABLED_CHANGED) ||
                            events.contains(Player.EVENT_REPEAT_MODE_CHANGED) ||
                            events.contains(Player.EVENT_VOLUME_CHANGED) ||
                            events.contains(Player.EVENT_POSITION_DISCONTINUITY)
                    )
                        persist(false)
                }

                override fun onMediaItemTransition(item: MediaItem?, reason: Int) {
                    if (player.playWhenReady && item != null) repository.played(item.mediaId)
                }
            }
        )
        val open =
            PendingIntent.getActivity(
                this,
                0,
                Intent(this, MainActivity::class.java),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        session =
            MediaLibrarySession.Builder(this, player, Callbacks()).setSessionActivity(open).build()
        // Restore metadata and position, paused. A headset/system Play remains an explicit command.
        scope.launch {
            val restore =
                withContext(Dispatchers.IO) {
                    repository.refresh()
                    restore()
                }
            if (player.mediaItemCount == 0 && restore.mediaItems.isNotEmpty()) {
                player.setMediaItems(
                    restore.mediaItems,
                    restore.startIndex,
                    restore.startPositionMs,
                )
                player.repeatMode = saved.getInt("repeat", Player.REPEAT_MODE_OFF)
                player.shuffleModeEnabled = saved.getBoolean("shuffle", false)
                player.volume = saved.getFloat("volume", 1f).coerceIn(0f, 1f)
                player.prepare()
                player.playWhenReady = false
            }
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaLibrarySession? =
        session

    private fun persist(queue: Boolean) {
        val edit =
            saved
                .edit()
                .putInt("index", player.currentMediaItemIndex.coerceAtLeast(0))
                .putLong("position", player.currentPosition.coerceAtLeast(0))
                .putFloat("volume", player.volume)
                .putInt("repeat", player.repeatMode)
                .putBoolean("shuffle", player.shuffleModeEnabled)
        if (queue) {
            val rows = JSONArray()
            for (i in 0 until player.mediaItemCount) {
                val item = player.getMediaItemAt(i)
                rows.put(
                    JSONObject()
                        .put("id", item.mediaId)
                        .put(
                            "entry",
                            item.requestMetadata.extras?.getString("entry") ?: capability(),
                        )
                )
            }
            edit.putString("queue", rows.toString())
        }
        edit.apply()
    }

    private fun restore(): MediaSession.MediaItemsWithStartPosition {
        val rows =
            runCatching { JSONArray(saved.getString("queue", "[]")) }.getOrDefault(JSONArray())
        val originalIndex = saved.getInt("index", 0)
        var index = 0
        val items = buildList {
            for (i in 0 until rows.length()) {
                val row = rows.getJSONObject(i)
                val song = repository.song(row.getString("id"))
                if (song?.available == true) {
                    if (i <= originalIndex) index = size
                    add(
                        song.media(
                            row.optString("entry").takeIf { it.matches(Regex("[a-f0-9]{64}")) }
                                ?: capability()
                        )
                    )
                }
            }
        }
        return MediaSession.MediaItemsWithStartPosition(
            items,
            index.coerceIn(0, (items.size - 1).coerceAtLeast(0)),
            saved.getLong("position", 0).coerceAtLeast(0),
        )
    }

    private fun folder(id: String, title: String) =
        MediaItem.Builder()
            .setMediaId(id)
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(title)
                    .setIsBrowsable(true)
                    .setIsPlayable(false)
                    .setMediaType(MediaMetadata.MEDIA_TYPE_FOLDER_MIXED)
                    .build()
            )
            .build()

    private fun children(id: String): List<MediaItem> {
        val library = repository.snapshots.value
        return when {
            id == "root" ->
                listOf(
                    folder("albums", "Albums"),
                    folder("artists", "Artists"),
                    folder("songs", "Songs"),
                    folder("collections", "Collections"),
                    folder("favorites", "Favorites"),
                )
            id == "albums" ->
                library.songs
                    .groupBy { it.albumId }
                    .map { (key, songs) -> folder("album:$key", songs.first().album) }
            id == "artists" ->
                library.songs
                    .groupBy { it.artist }
                    .map { (name, _) -> folder("artist:$name", name) }
            id == "collections" ->
                library.collections.map { folder("collection:${it.id}", it.name) }
            id.startsWith("collection:") ->
                library.collections
                    .find { it.id == id.removePrefix("collection:") }
                    ?.songIds
                    .orEmpty()
                    .mapNotNull(repository::song)
                    .filter { it.available }
                    .map { it.media() }
            else -> {
                val songs =
                    when {
                        id == "favorites" -> library.songs.filter { it.favorite }
                        id.startsWith("album:") ->
                            library.songs.filter { it.albumId == id.removePrefix("album:") }
                        id.startsWith("artist:") ->
                            library.songs.filter { it.artist == id.removePrefix("artist:") }
                        id == "songs" -> library.songs
                        else -> emptyList()
                    }
                songs.filter { it.available }.map { it.media() }
            }
        }
    }

    private fun paginate(items: List<MediaItem>, page: Int, size: Int): List<MediaItem> {
        if (page < 0 || size <= 0) return emptyList()
        val start = page.toLong() * size
        if (start >= items.size) return emptyList()
        return items.drop(start.toInt()).take(size.coerceAtMost(500))
    }

    private inner class Callbacks : MediaLibrarySession.Callback {
        override fun onConnectAsync(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
        ): ListenableFuture<MediaSession.ConnectionResult> {
            if (
                controller.packageName != packageName &&
                    !controller.isTrusted &&
                    !session.isMediaNotificationController(controller)
            )
                return Futures.immediateFuture(MediaSession.ConnectionResult.reject())
            val commands =
                MediaSession.ConnectionResult.DEFAULT_SESSION_AND_LIBRARY_COMMANDS.buildUpon()
                    .add(SessionCommand("musicos.audio_session", Bundle.EMPTY))
                    .build()
            return Futures.immediateFuture(
                MediaSession.ConnectionResult.accept(
                    commands,
                    MediaSession.ConnectionResult.DEFAULT_PLAYER_COMMANDS,
                )
            )
        }

        override fun onGetLibraryRoot(
            session: MediaLibrarySession,
            browser: MediaSession.ControllerInfo,
            params: LibraryParams?,
        ) = Futures.immediateFuture(LibraryResult.ofItem(folder("root", "MusicOS"), params))

        override fun onGetChildren(
            session: MediaLibrarySession,
            browser: MediaSession.ControllerInfo,
            parentId: String,
            page: Int,
            pageSize: Int,
            params: LibraryParams?,
        ): ListenableFuture<LibraryResult<ImmutableList<MediaItem>>> =
            Futures.immediateFuture(
                LibraryResult.ofItemList(paginate(children(parentId), page, pageSize), params)
            )

        override fun onGetItem(
            session: MediaLibrarySession,
            browser: MediaSession.ControllerInfo,
            mediaId: String,
        ): ListenableFuture<LibraryResult<MediaItem>> {
            val song = repository.song(mediaId)
            return Futures.immediateFuture(
                if (song?.available == true) LibraryResult.ofItem(song.media(), null)
                else LibraryResult.ofError(androidx.media3.session.SessionError.ERROR_BAD_VALUE)
            )
        }

        override fun onSearch(
            session: MediaLibrarySession,
            browser: MediaSession.ControllerInfo,
            query: String,
            params: LibraryParams?,
        ): ListenableFuture<LibraryResult<Void>> {
            session.notifySearchResultChanged(browser, query, matches(query).size, params)
            return Futures.immediateFuture(LibraryResult.ofVoid())
        }

        private fun matches(query: String) =
            repository.snapshots.value.songs
                .filter {
                    it.available &&
                        "${it.title} ${it.artist} ${it.album}"
                            .contains(query.take(500), ignoreCase = true)
                }
                .map { it.media() }

        override fun onGetSearchResult(
            session: MediaLibrarySession,
            browser: MediaSession.ControllerInfo,
            query: String,
            page: Int,
            pageSize: Int,
            params: LibraryParams?,
        ): ListenableFuture<LibraryResult<ImmutableList<MediaItem>>> =
            Futures.immediateFuture(
                LibraryResult.ofItemList(paginate(matches(query), page, pageSize), params)
            )

        override fun onAddMediaItems(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            mediaItems: List<MediaItem>,
        ): ListenableFuture<List<MediaItem>> {
            return try {
                require(mediaItems.size <= 10000)
                Futures.immediateFuture(
                    mediaItems.flatMap { item ->
                        val query = item.requestMetadata.searchQuery
                        if (!query.isNullOrBlank()) return@flatMap matches(query)
                        if (
                            item.mediaId.startsWith("album:") ||
                                item.mediaId.startsWith("artist:") ||
                                item.mediaId.startsWith("collection:") ||
                                item.mediaId == "favorites" ||
                                item.mediaId == "songs"
                        )
                            return@flatMap children(item.mediaId)
                        val song = repository.song(item.mediaId)
                        require(song?.available == true) { "This song is not available offline" }
                        listOf(
                            song.media(
                                item.requestMetadata.extras?.getString("entry")?.takeIf {
                                    it.matches(Regex("[a-f0-9]{64}"))
                                } ?: capability()
                            )
                        )
                    }
                )
            } catch (error: Exception) {
                Futures.immediateFailedFuture(error)
            }
        }

        override fun onPlaybackResumption(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            isForPlayback: Boolean,
        ): ListenableFuture<MediaSession.MediaItemsWithStartPosition> {
            val result = SettableFuture.create<MediaSession.MediaItemsWithStartPosition>()
            scope.launch {
                try {
                    val items =
                        withContext(Dispatchers.IO) {
                            repository.refresh()
                            restore()
                        }
                    require(items.mediaItems.isNotEmpty()) { "Choose offline music first" }
                    result.set(items)
                } catch (error: Exception) {
                    result.setException(error)
                }
            }
            return result
        }

        override fun onSetRating(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            mediaId: String,
            rating: Rating,
        ): ListenableFuture<SessionResult> {
            val song = repository.song(mediaId)
            if (song == null || rating !is HeartRating || !rating.isRated)
                return Futures.immediateFuture(
                    SessionResult(androidx.media3.session.SessionError.ERROR_BAD_VALUE)
                )
            if (song.favorite != rating.isHeart) repository.favorite(song.id)
            return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
        }

        override fun onCustomCommand(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            command: SessionCommand,
            args: Bundle,
        ): ListenableFuture<SessionResult> {
            if (
                command.customAction == "musicos.audio_session" &&
                    controller.packageName == packageName
            )
                return Futures.immediateFuture(
                    SessionResult(
                        SessionResult.RESULT_SUCCESS,
                        Bundle().apply { putInt("session", player.audioSessionId) },
                    )
                )
            return Futures.immediateFuture(
                SessionResult(androidx.media3.session.SessionError.ERROR_NOT_SUPPORTED)
            )
        }
    }

    override fun onDestroy() {
        persist(true)
        handler.removeCallbacksAndMessages(null)
        scope.cancel()
        session?.release()
        player.release()
        session = null
        super.onDestroy()
    }
}

fun Song.media(entry: String = capability()): MediaItem =
    MediaItem.Builder()
        .setMediaId(id)
        .setUri(Uri.fromFile(File(requireNotNull(localFile))))
        .setMimeType(mime)
        .setRequestMetadata(
            MediaItem.RequestMetadata.Builder()
                .setExtras(Bundle().apply { putString("entry", entry) })
                .build()
        )
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(title)
                .setArtist(artist)
                .setAlbumTitle(album)
                .setAlbumArtist(artist)
                .setArtworkUri(artworkFile?.let { Uri.fromFile(File(it)) })
                .setTrackNumber(trackNumber.takeIf { it > 0 })
                .setDiscNumber(discNumber.takeIf { it > 0 })
                .setGenre(genre)
                .setDurationMs(duration)
                .setIsBrowsable(false)
                .setIsPlayable(true)
                .setMediaType(MediaMetadata.MEDIA_TYPE_MUSIC)
                .setUserRating(HeartRating(favorite))
                .build()
        )
        .build()
