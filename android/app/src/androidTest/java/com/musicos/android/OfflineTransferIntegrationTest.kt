package com.musicos.android

import android.content.ComponentName
import android.os.Build
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkInfo
import androidx.work.WorkManager
import androidx.work.workDataOf
import java.io.File
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
class OfflineTransferIntegrationTest {
    @get:Rule val compose = createEmptyComposeRule()
    @get:Rule
    val permissions =
        GrantPermissionRule.grant(
            *buildList {
                    if (Build.VERSION.SDK_INT >= 37) add("android.permission.ACCESS_LOCAL_NETWORK")
                    if (Build.VERSION.SDK_INT >= 33) add("android.permission.POST_NOTIFICATIONS")
                }
                .toTypedArray()
        )

    @Test
    fun approvedMacCopiesVerifiedMusicAndPhonePlaysAfterServerStops() =
        runBlocking<Unit> {
            val path = InstrumentationRegistry.getArguments().getString("pairingFixture")
            assumeTrue("Run against the isolated Rust TLS fixture", path != null)
            val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
            val scenario = ActivityScenario.launch(MainActivity::class.java)
            try {
                val link = PairLink.parse(JSONObject(File(path!!).readText()).getString("link"))
                val mac =
                    PairedMac(
                        link.server,
                        "Fixture Mac",
                        link.host,
                        link.port,
                        link.fingerprint,
                        capability(),
                        capability(),
                    )
                val client = WifiClient(mac)
                client.pair(link.ticket, "Android emulator offline test")
                var approved = false
                for (i in 0..50) {
                    if (client.pairStatus().getString("state") == "approved") {
                        approved = true
                        break
                    }
                    delay(100)
                }
                assertTrue(approved)
                SecurePairingStore(app).put(mac)
                val manifest = client.manifest()
                assertEquals(1, manifest.tracks.size)
                assertEquals(2, manifest.collections.single().songIds.size)
                val request =
                    OneTimeWorkRequestBuilder<SyncWorker>()
                        .setInputData(workDataOf("server" to mac.id))
                        .build()
                val work = WorkManager.getInstance(app)
                work.enqueue(request).result.get(5, TimeUnit.SECONDS)
                var result: WorkInfo? = null
                for (i in 0..300) {
                    result = work.getWorkInfoById(request.id).get(5, TimeUnit.SECONDS)
                    if (result?.state?.isFinished == true) break
                    delay(100)
                }
                assertEquals(
                    result?.outputData?.getString("error"),
                    WorkInfo.State.SUCCEEDED,
                    result?.state,
                )
                val song = app.repository.song(manifest.tracks.single().id)!!
                assertTrue(song.available)
                assertTrue(WifiClient.verified(File(song.localFile!!), song.hash, song.size))
                assertTrue(File(song.artworkFile!!).isFile)
                assertEquals(
                    listOf(song.id, song.id),
                    app.repository.snapshots.value.collections
                        .find { it.serverId == mac.id }!!
                        .songIds,
                )
                delay(1500)
                assertTrue(
                    "The synthetic Mac has stopped sharing",
                    runCatching { client.manifest() }.isFailure,
                )
                var connection: MediaController? = null
                val instrumentation = InstrumentationRegistry.getInstrumentation()
                instrumentation.runOnMainSync {
                    val future =
                        MediaController.Builder(
                                app,
                                SessionToken(app, ComponentName(app, PlaybackService::class.java)),
                            )
                            .buildAsync()
                    future.addListener(
                        { connection = future.get() },
                        androidx.core.content.ContextCompat.getMainExecutor(app),
                    )
                }
                compose.waitUntil(10000) { connection != null }
                val controller = connection!!
                var playing = false
                instrumentation.runOnMainSync {
                    controller.setMediaItems(listOf(song.media(), song.media()))
                    controller.prepare()
                    controller.play()
                }
                compose.waitUntil(10000) {
                    instrumentation.runOnMainSync { playing = controller.isPlaying }
                    playing
                }
                compose.waitUntil(10000) {
                    app.getSystemService(android.app.NotificationManager::class.java)
                        .activeNotifications
                        .any {
                            it.notification.category == android.app.Notification.CATEGORY_TRANSPORT
                        }
                }
                instrumentation.runOnMainSync {
                    assertEquals(2, controller.mediaItemCount)
                    assertNotNull(controller.mediaMetadata.artworkUri)
                    controller.pause()
                    controller.clearMediaItems()
                    controller.release()
                }
            } finally {
                scenario.close()
            }
        }
}
