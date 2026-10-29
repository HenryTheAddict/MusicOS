package com.musicos.android

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.StatFs
import androidx.core.app.NotificationCompat
import androidx.work.CoroutineWorker
import androidx.work.ForegroundInfo
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import java.io.IOException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class SyncWorker(context: Context, parameters: WorkerParameters) :
    CoroutineWorker(context, parameters) {
    private val repository = (applicationContext as MusicOSApp).repository
    private var lastProgress = 0L

    override suspend fun getForegroundInfo() = notification("Preparing offline music", 0, 0)

    private fun notification(text: String, done: Long, total: Long): ForegroundInfo {
        val manager = applicationContext.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                "offline-sync",
                "Offline music transfers",
                NotificationManager.IMPORTANCE_LOW,
            )
        )
        val open =
            PendingIntent.getActivity(
                applicationContext,
                0,
                Intent(applicationContext, MainActivity::class.java).putExtra("open_devices", true),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        val builder =
            NotificationCompat.Builder(applicationContext, "offline-sync")
                .setSmallIcon(android.R.drawable.stat_sys_download)
                .setContentTitle("musicOS · Copying for offline listening")
                .setContentText(text)
                .setContentIntent(open)
                .setOnlyAlertOnce(true)
                .setOngoing(true)
                .addAction(
                    android.R.drawable.ic_menu_close_clear_cancel,
                    "Cancel",
                    WorkManager.getInstance(applicationContext).createCancelPendingIntent(id),
                )
        if (total > 0)
            builder.setProgress(100, ((done * 100) / total).toInt().coerceIn(0, 100), false)
        else builder.setProgress(0, 0, true)
        return if (Build.VERSION.SDK_INT >= 29)
            ForegroundInfo(2001, builder.build(), ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        else ForegroundInfo(2001, builder.build())
    }

    override suspend fun doWork(): Result =
        withContext(Dispatchers.IO) {
            try {
                val serverId =
                    inputData.getString("server")
                        ?: return@withContext Result.failure(
                            workDataOf("error" to "Choose a paired Mac")
                        )
                var mac =
                    SecurePairingStore(applicationContext).all().find { it.id == serverId }
                        ?: return@withContext Result.failure(
                            workDataOf("error" to "Pair this Mac again")
                        )
                val connectivity = applicationContext.getSystemService(android.net.ConnectivityManager::class.java)
                val localNetwork = connectivity.allNetworks.any { network -> connectivity.getNetworkCapabilities(network)?.let { it.hasTransport(android.net.NetworkCapabilities.TRANSPORT_WIFI) || it.hasTransport(android.net.NetworkCapabilities.TRANSPORT_ETHERNET) } == true }
                require(localNetwork) { "Join the same Wi-Fi as your Mac, then tap Copy music again." }
                setForeground(notification("Connecting to ${mac.name}", 0, 0))
                var client = WifiClient(mac)
                val waiting: suspend (String) -> Unit = { message ->
                    setProgress(workDataOf("message" to message))
                    setForeground(notification(message, 0, 0))
                }
                val manifest =
                    try {
                        client.manifest(waiting)
                    } catch (first: IOException) {
                        val found = MacDiscovery(applicationContext).find(mac.id, mac.fingerprint)
                        if (found == null) throw first
                        mac = mac.copy(host = found.host, port = found.port)
                        SecurePairingStore(applicationContext).put(mac)
                        client = WifiClient(mac)
                        client.manifest(waiting)
                    }
                val needed =
                    manifest.tracks
                        .filter {
                            repository.objectFile(it.hash).let { file -> !file.isFile || file.length() != it.size }
                        }
                        .sumOf { it.size }
                require(
                    StatFs(applicationContext.filesDir.path).availableBytes >
                        needed + 64L * 1024 * 1024
                ) {
                    "Not enough space on this phone. Free some storage and retry."
                }
                var completeBytes = 0L
                val files = mutableMapOf<String, java.io.File>()
                val art = mutableMapOf<String, java.io.File>()
                var published = 0
                var lastPublished = android.os.SystemClock.elapsedRealtime()
                for ((index, song) in manifest.tracks.withIndex()) {
                    val before = completeBytes
                    val target =
                        client.download(
                            "tracks/${song.remoteId}",
                            repository.objectFile(song.hash),
                            song.hash,
                            song.size,
                        ) { written ->
                            val now = android.os.SystemClock.elapsedRealtime()
                            if (now - lastProgress > 400) {
                                lastProgress = now
                                setProgress(
                                    workDataOf(
                                        "done" to before + written,
                                        "total" to manifest.bytes,
                                        "song" to song.title,
                                        "message" to "Copying ${index + 1} of ${manifest.tracks.size} · ${song.title}",
                                        "count" to index,
                                        "songs" to manifest.tracks.size,
                                    )
                                )
                                setForeground(
                                    notification(song.title, before + written, manifest.bytes)
                                )
                            }
                        }
                    files[song.id] = target
                    completeBytes += song.size
                    song.artwork?.let { hash ->
                        if (hash !in art) {
                            art[hash] =
                                client.download(
                                    "artwork/$hash",
                                    repository.artworkFile(hash),
                                    hash,
                                    null,
                                ) {}
                        }
                    }
                    val publishNow = android.os.SystemClock.elapsedRealtime()
                    if (files.size - published >= 8 || publishNow - lastPublished >= 2000) {
                        repository.import(manifest.copy(tracks = manifest.tracks.filter { files.containsKey(it.id) }, collections = emptyList()), files, art)
                        published = files.size
                        lastPublished = publishNow
                    }
                    setProgress(
                        workDataOf(
                            "done" to completeBytes,
                            "total" to manifest.bytes,
                            "song" to song.title,
                                        "message" to "Copying ${index + 1} of ${manifest.tracks.size} · ${song.title}",
                            "count" to index + 1,
                            "songs" to manifest.tracks.size,
                        )
                    )
                }
                setForeground(
                    notification("Saving verified library", manifest.bytes, manifest.bytes)
                )
                repository.import(manifest, files, art)
                client.complete(manifest)
                Result.success(
                    workDataOf("songs" to manifest.tracks.size, "bytes" to manifest.bytes)
                )
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: IOException) {
                if (
                    runAttemptCount < 4 &&
                        (error !is SyncHttpError ||
                            error.code >= 500 ||
                            error.code == 429 ||
                            error.code == 416)
                )
                    Result.retry()
                else
                    Result.failure(
                        workDataOf(
                            "error" to (error.message ?: "Reconnect to the same Wi-Fi and retry.")
                        )
                    )
            } catch (error: Exception) {
                Result.failure(
                    workDataOf(
                        "error" to
                            (error.message
                                ?: "The Mac is unavailable. Reconnect to the same Wi-Fi and retry.")
                    )
                )
            }
        }
}
