package com.musicos.android

import android.graphics.Bitmap
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PlayerPresentationTest {
    @get:Rule val compose = createComposeRule()

    @Test fun playerKeepsIdentityAndTransportTogetherAndRetainsDuplicateQueueEntries() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val store = ViewModelStore()
        val model = ViewModelProvider(store, ViewModelProvider.AndroidViewModelFactory(app))[PhoneModel::class.java]
        val cover = File(app.cacheDir, "player-presentation-cover.png")
        InstrumentationRegistry.getInstrumentation().context.assets.open("player-cover.png").use { input -> cover.outputStream().use { input.copyTo(it) } }
        val song = Song("presentation/1", "a".repeat(64), 1, "A long song title for a small screen", "musicOS", "Offline favorites", "album", "Electronic", 2026, 1, 1, 180000, 0, "b".repeat(64), "audio/wav", null, false, null, cover.path)
        val state = Playing(song = song, position = 30000, duration = 180000,
            queue = listOf(QueueEntry("first", song, 0), QueueEntry("duplicate", song, 1)))
        var queue by mutableStateOf(false)
        var dark by mutableStateOf(false)
        var short by mutableStateOf(false)
        var closed = false
        try {
            compose.setContent {
                MusicTheme(dark) {
                    Box(Modifier.width(360.dp).height(if(short) 480.dp else 720.dp)
                        .background(LocalMusicColors.current.surface)) {
                        PlayerPage(model, state, queue, { queue = it }, { closed = true })
                    }
                }
            }
            for (theme in listOf(false, true)) for (small in listOf(false, true)) {
                compose.runOnIdle { dark = theme; short = small; queue = false }
                compose.onNodeWithContentDescription("Play").assertIsDisplayed()
                compose.onNodeWithContentDescription("Next").assertIsDisplayed()
                compose.onNodeWithContentDescription("Favorite playing song").assertIsDisplayed()
                val art = compose.onNodeWithTag("player-artwork").getUnclippedBoundsInRoot()
                val identity = compose.onNodeWithTag("player-identity").getUnclippedBoundsInRoot()
                assertEquals((art.right - art.left).value, (art.bottom - art.top).value, 1f)
                assertTrue(identity.top.value - art.bottom.value < 20f)
                compose.onNodeWithContentDescription("Volume controls").performClick()
                compose.onNodeWithContentDescription("Listening volume").assertIsDisplayed()
                val volume = compose.onNodeWithContentDescription("Listening volume").getUnclippedBoundsInRoot()
                assertTrue("Volume is fully inside the player", volume.bottom.value <= (if(small) 480f else 720f))
                compose.waitUntil(5000) { cover.exists() }
                compose.waitForIdle()
                val image = compose.onRoot().captureToImage().asAndroidBitmap()
                val output = File(app.getExternalFilesDir(null), "qa/player-${if(theme) "dark" else "light"}-${if(small) "short" else "tall"}.png")
                output.parentFile!!.mkdirs()
                output.outputStream().use { image.compress(Bitmap.CompressFormat.PNG, 100, it) }
                compose.onNodeWithContentDescription("Volume controls").performClick()
                compose.onNodeWithText("Up next").performClick()
                compose.onAllNodesWithText(song.title).assertCountEquals(2)
                compose.onNodeWithText("Now playing").performClick()
                compose.onNodeWithText("Up next").performClick()
                compose.onAllNodesWithText(song.title).assertCountEquals(2)
            }
            compose.onNodeWithContentDescription("Close player").performClick()
            compose.runOnIdle { assertTrue(closed) }
        } finally { compose.runOnIdle { store.clear() }; cover.delete() }
    }
}
