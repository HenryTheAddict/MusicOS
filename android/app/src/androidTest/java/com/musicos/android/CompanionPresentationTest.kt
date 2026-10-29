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
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CompanionPresentationTest {
    @get:Rule val compose = createComposeRule()

    @Test fun pairedMacComesBeforePairingAndUsesFilledActionsInBothThemes() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val pairings = SecurePairingStore(app)
        val mac = PairedMac("d".repeat(64), "Henry’s Mac", "192.168.1.2", 8443, "a".repeat(64), "b".repeat(64), "c".repeat(64))
        pairings.put(mac)
        val store = ViewModelStore()
        val model = ViewModelProvider(store, ViewModelProvider.AndroidViewModelFactory(app))[PhoneModel::class.java]
        var dark by mutableStateOf(false)
        try {
            compose.setContent {
                MusicTheme(dark) {
                    Box(Modifier.width(360.dp).height(720.dp).background(LocalMusicColors.current.surface)) {
                        DevicesPage(model, "", {})
                    }
                }
            }
            compose.waitUntil(5000) { model.macs.value.any { it.id == mac.id } }
            for (theme in listOf(false, true)) {
                compose.runOnIdle { dark = theme }
                compose.onNodeWithText(mac.name).assertIsDisplayed()
                compose.onNodeWithText("Copy music").assertIsDisplayed()
                compose.onNodeWithText("Scan Mac code").assertDoesNotExist()
                val paired = compose.onNodeWithText(mac.name).getUnclippedBoundsInRoot()
                val pairing = compose.onNodeWithText("Another Mac?").getUnclippedBoundsInRoot()
                assertTrue(paired.top < pairing.top)
                val output = File(app.getExternalFilesDir(null), "qa/devices-${if(theme) "dark" else "light"}.png")
                output.parentFile!!.mkdirs()
                compose.onRoot().captureToImage().asAndroidBitmap().let { image -> output.outputStream().use { image.compress(Bitmap.CompressFormat.PNG, 100, it) } }
                compose.onNodeWithText("Add", substring = false).performClick()
                compose.onNodeWithText("Scan Mac code").performScrollTo().assertIsDisplayed()
                compose.onNodeWithText("Close", substring = false).performScrollTo().performClick()
            }
        } finally { compose.runOnIdle { store.clear() }; pairings.remove(mac.id) }
    }

    @Test fun privateCopyReceiptRejectsSameSizeMutationAndReplacement() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val file = File(app.cacheDir, "receipt-device-fixture.audio").apply { writeText("music") }
        val receipt = File(file.path + ".verified")
        val hash = sha256(file.readBytes())
        try {
            assertTrue(CopyVerification.verified(file, hash, 5))
            assertTrue("Real Android records nanosecond file identity", receipt.isFile)
            assertTrue(CopyVerification.verified(file, hash, 5))
            file.writeText("other")
            assertFalse(CopyVerification.verified(file, hash, 5))
            file.delete(); file.writeText("other")
            assertFalse(CopyVerification.verified(file, hash, 5))
            file.writeText("music")
            assertTrue(CopyVerification.verified(file, hash, 5))
        } finally { file.delete(); receipt.delete() }
    }

    @Test fun artworkDecodeIsBoundedAndReusedWithoutHdrGainMaps() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        val file = File(app.cacheDir, "large-cover-fixture.png")
        val source = Bitmap.createBitmap(2560, 2560, Bitmap.Config.ARGB_8888)
        source.eraseColor(android.graphics.Color.rgb(184, 54, 96))
        file.outputStream().use { source.compress(Bitmap.CompressFormat.PNG, 100, it) }; source.recycle()
        try {
            val small = CompanionArtwork.load(file.path, 256)!!
            assertTrue(small.width <= 256 && small.height <= 256)
            assertSame(small, CompanionArtwork.load(file.path, 256))
            val player = CompanionArtwork.load(file.path, 1024)!!
            assertTrue(player.width <= 1024 && player.height <= 1024)
            assertEquals(android.graphics.ColorSpace.Named.SRGB.name, player.colorSpace?.name?.replace("sRGB IEC61966-2.1", "SRGB"))
            if(android.os.Build.VERSION.SDK_INT >= 34) assertFalse(player.hasGainmap())
        } finally { file.delete() }
    }

    @Test fun exportedSketchIconsRenderStrokesWithoutExportBackground() {
        val app = ApplicationProvider.getApplicationContext<MusicOSApp>()
        for (name in listOf("Phone", "Download", "Downloaded")) {
            val source = app.assets.open("icons/$name.svg").bufferedReader().use { it.readText() }
            val svg = com.caverock.androidsvg.SVG.getFromString(themedSketchSvg(source, "#12202a", "#a6d7e8"))
            svg.documentWidth = 32f; svg.documentHeight = 32f
            val image = Bitmap.createBitmap(32, 32, Bitmap.Config.ARGB_8888)
            android.graphics.Canvas(image).drawPicture(svg.renderToPicture())
            val pixels = IntArray(1024); image.getPixels(pixels, 0, 32, 0, 0, 32, 32)
            assertTrue("$name contains visible supplied strokes", pixels.count { android.graphics.Color.alpha(it) > 0 } > 80)
            assertEquals("No white export background", 0, android.graphics.Color.alpha(image.getPixel(0, 0)))
            image.recycle()
        }
    }
}
