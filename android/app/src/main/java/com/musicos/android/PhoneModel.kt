package com.musicos.android

import android.app.Application
import android.content.ComponentName
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import java.util.UUID
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class QueueEntry(val entry: String, val song: Song?, val index: Int)

data class Playing(
    val song: Song? = null,
    val playing: Boolean = false,
    val position: Long = 0,
    val duration: Long = 0,
    val volume: Float = 1f,
    val shuffle: Boolean = false,
    val repeat: Int = Player.REPEAT_MODE_OFF,
    val queue: List<QueueEntry> = emptyList(),
    val current: Int = 0,
    val error: String? = null,
)

@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
class PhoneModel(app: Application) : AndroidViewModel(app) {
    val repository = (app as MusicOSApp).repository
    val library = repository.snapshots
    val work = WorkManager.getInstance(app)
    private val pairings = SecurePairingStore(app)
    private val playback = MutableStateFlow(Playing())
    val playing = playback.asStateFlow()
    private val paired = MutableStateFlow<List<PairedMac>>(emptyList())
    val macs = paired.asStateFlow()
    val message = MutableStateFlow<String?>(null)
    val pairing = MutableStateFlow(false)
    val pairingStatus = MutableStateFlow("")
    val jobs = MutableStateFlow<Map<String, UUID>>(emptyMap())
    private val jobSettings = app.getSharedPreferences("sync-jobs-v1", 0)
    private val future =
        MediaController.Builder(
                app,
                SessionToken(app, ComponentName(app, PlaybackService::class.java)),
            )
            .buildAsync()
    var controller: MediaController? = null
        private set

    private val listener =
        object : Player.Listener {
            override fun onEvents(player: Player, events: Player.Events) {
                acknowledge(
                    events.contains(Player.EVENT_TIMELINE_CHANGED) ||
                        events.contains(Player.EVENT_MEDIA_ITEM_TRANSITION)
                )
            }
        }

    init {
        future.addListener(
            {
                runCatching { future.get() }
                    .onSuccess {
                        controller = it
                        it.addListener(listener)
                        acknowledge(true)
                    }
                    .onFailure {
                        message.value =
                            "Could not connect to Android playback. Reopen MusicOS to retry."
                    }
            },
            ContextCompat.getMainExecutor(app),
        )
        viewModelScope.launch(Dispatchers.IO) {
            runCatching { pairings.all() }
                .onSuccess { list ->
                    paired.value = list
                    jobs.value =
                        list
                            .mapNotNull { mac ->
                                jobSettings.getString(mac.id, null)?.let { value ->
                                    runCatching { mac.id to UUID.fromString(value) }.getOrNull()
                                }
                            }
                            .toMap()
                }
                .onFailure {
                    message.value =
                        "Android could not unlock the saved pairings. Your offline music is safe; pair the Mac again."
                }
        }
        viewModelScope.launch { library.collect { acknowledge(true) } }
    }

    private fun acknowledge(queueChanged: Boolean = false) {
        val p = controller ?: return
        val previous = playback.value
        val queue =
            if (queueChanged)
                (0 until p.mediaItemCount).map { index ->
                    val item = p.getMediaItemAt(index)
                    QueueEntry(
                        item.requestMetadata.extras?.getString("entry") ?: "$index/${item.mediaId}",
                        repository.song(item.mediaId),
                        index,
                    )
                }
            else previous.queue
        playback.value =
            Playing(
                repository.song(p.currentMediaItem?.mediaId.orEmpty()),
                p.isPlaying,
                p.currentPosition.coerceAtLeast(0),
                p.duration.takeIf { it > 0 }
                    ?: repository.song(p.currentMediaItem?.mediaId.orEmpty())?.duration
                    ?: 0,
                p.volume,
                p.shuffleModeEnabled,
                p.repeatMode,
                queue,
                p.currentMediaItemIndex.coerceAtLeast(0),
                p.playerError?.let {
                    "This song could not play on this device. Try another song or a compatible audio format."
                },
            )
    }

    fun position() {
        val p = controller ?: return
        playback.value = playback.value.copy(position = p.currentPosition.coerceAtLeast(0))
    }

    fun play(songs: List<Song>, start: Int = 0) {
        val playable = songs.filter { it.available }
        if (playable.isEmpty()) {
            message.value = "Copy this music from the Mac before playing it."
            return
        }
        val p = controller ?: return
        p.setMediaItems(playable.map { it.media() }, start.coerceIn(0, playable.lastIndex), 0)
        p.prepare()
        p.play()
    }

    fun toggle() {
        controller?.let {
            if (it.isPlaying) it.pause()
            else if (it.mediaItemCount > 0) {
                it.prepare()
                it.play()
            }
        }
    }

    fun next() {
        controller?.seekToNextMediaItem()
    }

    fun previous() {
        controller?.seekToPrevious()
    }

    fun seek(position: Long) {
        controller?.seekTo(position)
    }

    fun volume(value: Float) {
        controller?.volume = value.coerceIn(0f, 1f)
    }

    fun shuffle() {
        controller?.let { it.shuffleModeEnabled = !it.shuffleModeEnabled }
    }

    fun repeat() {
        controller?.let { it.repeatMode = (it.repeatMode + 1) % 3 }
    }

    fun select(index: Int) {
        controller?.seekToDefaultPosition(index)
        controller?.play()
    }

    fun move(from: Int, to: Int) {
        controller?.moveMediaItem(from, to)
    }

    fun remove(index: Int) {
        controller?.removeMediaItem(index)
    }

    fun clearUpcoming() {
        controller?.let {
            if (it.currentMediaItemIndex + 1 < it.mediaItemCount)
                it.removeMediaItems(it.currentMediaItemIndex + 1, it.mediaItemCount)
        }
    }

    fun enqueue(song: Song, next: Boolean = false) {
        if (!song.available) return
        controller?.let { p ->
            p.addMediaItem(
                if (next) (p.currentMediaItemIndex + 1).coerceIn(0, p.mediaItemCount)
                else p.mediaItemCount,
                song.media(),
            )
            if (p.playbackState == Player.STATE_IDLE) p.prepare()
        }
    }

    fun pair(link: String) {
        if (pairing.value) return
        viewModelScope.launch {
            pairing.value = true
            try {
                val parsed = PairLink.parse(link)
                val pending =
                    PairedMac(
                        parsed.server,
                        "MusicOS",
                        parsed.host,
                        parsed.port,
                        parsed.fingerprint,
                        capability(),
                        capability(),
                    )
                val client = WifiClient(pending)
                val response =
                    client.pair(parsed.ticket, "${Build.MANUFACTURER} ${Build.MODEL}".take(60))
                val mac = pending.copy(name = response.optString("server_name", "MusicOS"))
                pairingStatus.value =
                    "Approve this phone in Devices on ${mac.name}. Request ${mac.clientId.take(6)}."
                var approved = false
                repeat(120) {
                    if (!approved) {
                        delay(1000)
                        if (client.pairStatus().getString("state") == "approved") {
                            withContext(Dispatchers.IO) { pairings.put(mac) }
                            paired.value = paired.value.filter { it.id != mac.id } + mac
                            approved = true
                            pairingStatus.value = "Paired. Choose Copy music to start."
                        }
                    }
                }
                if (!approved) error("Pairing expired. Create another link on the Mac.")
            } catch (cancelled: kotlinx.coroutines.CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                message.value = error.message ?: "Pairing failed. Create another link on the Mac."
                pairingStatus.value = ""
            } finally {
                pairing.value = false
            }
        }
    }

    fun sync(mac: PairedMac) {
        val request =
            OneTimeWorkRequestBuilder<SyncWorker>()
                .setInputData(workDataOf("server" to mac.id))
                .setConstraints(
                    Constraints.Builder()
                        .setRequiredNetworkType(NetworkType.UNMETERED)
                        .setRequiresStorageNotLow(true)
                        .build()
                )
                .setBackoffCriteria(
                    androidx.work.BackoffPolicy.EXPONENTIAL,
                    15,
                    java.util.concurrent.TimeUnit.SECONDS,
                )
                .build()
        work.enqueueUniqueWork("offline-${mac.id}", ExistingWorkPolicy.REPLACE, request)
        jobs.value = jobs.value + (mac.id to request.id)
        jobSettings.edit().putString(mac.id, request.id.toString()).apply()
    }

    fun cancel(mac: PairedMac) {
        jobs.value[mac.id]?.let { work.cancelWorkById(it) }
    }

    fun forget(mac: PairedMac) {
        cancel(mac)
        viewModelScope.launch(Dispatchers.IO) {
            pairings.remove(mac.id)
            paired.value = paired.value.filter { it.id != mac.id }
        }
    }

    override fun onCleared() {
        controller?.removeListener(listener)
        MediaController.releaseFuture(future)
        super.onCleared()
    }
}
