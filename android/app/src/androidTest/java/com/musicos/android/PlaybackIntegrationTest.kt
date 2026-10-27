package com.musicos.android

import android.content.ComponentName
import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.lifecycle.Lifecycle
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
class PlaybackIntegrationTest {
    @get:Rule val compose = createEmptyComposeRule()
    private val app
        get() = ApplicationProvider.getApplicationContext<MusicOSApp>()

    private val instrument
        get() = InstrumentationRegistry.getInstrumentation()

    private fun main(work: () -> Unit) = instrument.runOnMainSync(work)

    private fun seed(): Song {
        val bytes =
            ByteBuffer.allocate(44 + 48000 * 2 * 30)
                .order(ByteOrder.LITTLE_ENDIAN)
                .apply {
                    put("RIFF".toByteArray())
                    putInt(capacity() - 8)
                    put("WAVEfmt ".toByteArray())
                    putInt(16)
                    putShort(1)
                    putShort(1)
                    putInt(48000)
                    putInt(96000)
                    putShort(2)
                    putShort(16)
                    put("data".toByteArray())
                    putInt(capacity() - 44)
                }
                .array()
        val server = "a".repeat(64)
        val hash = sha256(bytes)
        val file = app.repository.objectFile(hash)
        file.writeBytes(bytes)
        val song =
            Song(
                "$server/1",
                server,
                1,
                "Offline fixture",
                "MusicOS",
                "Copied over Wi-Fi",
                "fixture",
                "Electronic",
                2026,
                1,
                1,
                30000,
                bytes.size.toLong(),
                hash,
                "audio/wav",
                null,
                false,
                file.path,
                null,
            )
        app.repository.import(
            SyncManifest(
                server,
                "Fixture Mac",
                "Henry",
                "b".repeat(64),
                listOf(song),
                listOf(
                    MusicCollection(
                        "$server/test",
                        server,
                        "Repeated songs",
                        "manual",
                        listOf(song.id, song.id),
                    )
                ),
                bytes.size.toLong(),
            ),
            mapOf(song.id to file),
            emptyMap(),
        )
        return app.repository.song(song.id)!!
    }

    private fun capture(name: String) {
        val image = compose.onRoot().captureToImage().asAndroidBitmap()
        val file = File(app.getExternalFilesDir(null), "qa/$name.png")
        file.parentFile!!.mkdirs()
        file.outputStream().use { image.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    @Test
    fun offlinePlaybackSharesOneSessionAndSurvivesBackground() {
        val song = seed()
        val scenario = ActivityScenario.launch(MainActivity::class.java)
        try {
            compose.waitUntil(10000) {
                compose.onAllNodesWithText("Offline fixture").fetchSemanticsNodes().isNotEmpty()
            }
            capture("android-home")
            compose.onNodeWithTag("nav-Library").performClick()
            compose.onNodeWithText("Songs", useUnmergedTree = true).performClick()
            compose.onNodeWithText("Offline fixture").assertExists()
            capture("android-library")
            var connected: MediaController? = null
            main {
                val future =
                    MediaController.Builder(
                            app,
                            SessionToken(app, ComponentName(app, PlaybackService::class.java)),
                        )
                        .buildAsync()
                future.addListener(
                    { connected = future.get() },
                    androidx.core.content.ContextCompat.getMainExecutor(app),
                )
            }
            compose.waitUntil(10000) { connected != null }
            val controller = connected!!
            var playing = false
            main {
                assertFalse(controller.isPlaying)
                controller.setMediaItems(listOf(song.media(), song.media(), song.media()))
                controller.prepare()
                controller.play()
            }
            compose.waitUntil(10000) {
                main { playing = controller.isPlaying }
                playing
            }
            main {
                assertEquals(3, controller.mediaItemCount)
                val entries =
                    (0..2).map {
                        controller.getMediaItemAt(it).requestMetadata.extras!!.getString("entry")
                    }
                assertEquals(3, entries.toSet().size)
                controller.moveMediaItem(2, 1)
                controller.removeMediaItem(2)
                assertEquals(2, controller.mediaItemCount)
            }
            compose.onNodeWithContentDescription("Up next").performClick()
            compose.onAllNodesWithText("Up next", useUnmergedTree = true).assertCountEquals(2)
            capture("android-queue")
            val expectedFavorite=!app.repository.song(song.id)!!.favorite
            app.repository.favorite(song.id)
            var favoriteAcknowledged = false
            compose.waitUntil(10000) {
                main {
                    favoriteAcknowledged =
                        (controller.mediaMetadata.userRating as? androidx.media3.common.HeartRating)
                            ?.isHeart == expectedFavorite
                }
                favoriteAcknowledged
            }
            main {
                assertTrue(controller.isPlaying)
                assertEquals(2, controller.mediaItemCount)
            }
            scenario.moveToState(Lifecycle.State.CREATED)
            main { assertTrue(controller.isPlaying) }
            scenario.moveToState(Lifecycle.State.RESUMED)
            main { controller.pause() }
            scenario.recreate()
            compose.waitUntil(10000) {
                compose.onAllNodesWithText("MusicOS").fetchSemanticsNodes().isNotEmpty()
            }
            main {
                assertFalse(controller.isPlaying)
                assertEquals(2, controller.mediaItemCount)
                controller.clearMediaItems()
                controller.release()
            }
        } finally {
            scenario.close()
        }
    }
}
