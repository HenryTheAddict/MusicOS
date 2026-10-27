package com.musicos.android

import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PresentationIntegrationTest {
    @get:Rule val compose = createEmptyComposeRule()

    @Test
    fun lightAndDarkKeepTheSketchNavigationAndRetainSearch() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val prefs = app.getSharedPreferences("appearance-v1", 0)
        val before = prefs.getString("theme", "system")
        try {
            for (theme in listOf("light", "dark")) {
                prefs.edit().putString("theme", theme).commit()
                val activity = ActivityScenario.launch(MainActivity::class.java)
                try {
                    compose.onNodeWithTag("nav-Search").performClick()
                    compose.onNode(hasSetTextAction()).performTextInput("Offline")
                    compose.onNodeWithTag("nav-Library").performClick()
                    compose.onNodeWithTag("nav-Home").performClick()
                    compose.onNodeWithTag("nav-Search").performClick()
                    compose.onNode(hasSetTextAction()).assertTextContains("Offline")
                    compose.onNodeWithTag("nav-Home").performClick()
                    compose.waitForIdle()
                    val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
                    val output = File(app.getExternalFilesDir(null), "qa/android-$theme.png")
                    output.parentFile!!.mkdirs()
                    output.outputStream().use {
                        bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
                    }
                    val color = bitmap.getPixel(bitmap.width / 2, 12)
                    if (theme == "dark") assertTrue(android.graphics.Color.blue(color) < 100)
                    else assertTrue(android.graphics.Color.blue(color) > 200)
                } finally {
                    activity.close()
                }
            }
        } finally {
            prefs.edit().putString("theme", before).commit()
        }
    }
}
