package com.musicos.android

import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PhoneLibraryPresentationTest {
    @get:Rule val compose = createEmptyComposeRule()
    @Test fun artworkShelvesAndLibraryViewsKeepNavigationAndOfflineCollections() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val cover = File(app.cacheDir,"library-cover.png")
        InstrumentationRegistry.getInstrumentation().context.assets.open("player-cover.png").use { input -> cover.outputStream().use { input.copyTo(it) } }
        val audio = File(app.cacheDir,"presentation.audio").apply { writeText("fixture") }
        val server = "e".repeat(64)
        val songs = (1L..4L).map { index -> Song("$server/$index",server,index, "Offline song $index", "musicOS", "Album $index", "$server/album-$index", "Electronic", 2026, 1, 1, 180000, audio.length(),sha256(audio.readBytes()),"audio/wav",sha256(cover.readBytes()),index==1L,audio.path,cover.path) }
        app.repository.import(SyncManifest(server,"Fixture Mac","Henry","f".repeat(64),songs,listOf(MusicCollection("$server/list",server,"Weekend favorites","manual",listOf(songs[0].id,songs[1].id))),songs.sumOf{it.size}),songs.associate{it.id to audio},mapOf(sha256(cover.readBytes()) to cover))
        val prefs=app.getSharedPreferences("appearance-v1",0);val before=prefs.getString("theme","system")
        try {for(theme in listOf("light","dark")) {
            prefs.edit().putString("theme",theme).commit()
            ActivityScenario.launch(MainActivity::class.java).use {
                compose.onNodeWithTag("nav-Home").performClick();compose.onNodeWithText("Collections").assertIsDisplayed()
                compose.onNodeWithText("Weekend favorites").assertIsDisplayed();capture(app,"home-$theme")
                compose.onNodeWithTag("nav-Library").performClick();compose.onNodeWithText("Album 1").assertIsDisplayed();capture(app,"library-$theme")
                compose.onNodeWithContentDescription("More library views").performClick();compose.onNodeWithText("Collections").performClick();compose.onNodeWithText("Weekend favorites · 2 songs").assertIsDisplayed()
                compose.onNodeWithTag("nav-Search").performClick();compose.onNode(hasSetTextAction()).performTextInput("Offline");compose.onNodeWithTag("nav-Home").performClick();compose.onNodeWithTag("nav-Search").performClick();compose.onNode(hasSetTextAction()).assertTextContains("Offline")
            }
        }} finally {prefs.edit().putString("theme",before).commit()}
    }
    private fun capture(app:MusicOSApp,name:String) {
        compose.waitForIdle();val bitmap=compose.onRoot().captureToImage().asAndroidBitmap();val output=File(app.getExternalFilesDir(null),"qa/$name.png");output.parentFile!!.mkdirs();output.outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
    }
}
