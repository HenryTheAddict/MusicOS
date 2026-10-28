package com.musicos.android

import android.content.ComponentName
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
class VorbisPlaybackTest {
    @get:Rule val compose = createEmptyComposeRule()
    @Test fun bothVorbisProfilesDecodeOfflineThroughTheExistingMediaSession() {
        val app=ApplicationProvider.getApplicationContext<MusicOSApp>()
        val instrument=InstrumentationRegistry.getInstrumentation()
        var controller:MediaController?=null
        ActivityScenario.launch(MainActivity::class.java).use {
            instrument.runOnMainSync {
                val future=MediaController.Builder(app,SessionToken(app,ComponentName(app,PlaybackService::class.java))).buildAsync()
                future.addListener({controller=future.get()},androidx.core.content.ContextCompat.getMainExecutor(app))
            }
            compose.waitUntil(10000){controller!=null}
            try {for(rate in listOf(256,320)) {
                val bytes=instrument.context.assets.open("vorbis$rate.ogg").use{it.readBytes()}
                val file=app.repository.objectFile(sha256(bytes)).apply{writeBytes(bytes)}
                val server="9".repeat(64)
                val song=Song("$server/$rate",server,rate.toLong(),"Vorbis $rate fixture","musicOS","Offline copies","vorbis","",2026,1,1,30000,bytes.size.toLong(),sha256(bytes),"audio/ogg",null,false,file.path,null)
                app.repository.import(SyncManifest(server,"Fixture Mac","Henry","f".repeat(64),listOf(song),emptyList(),song.size),mapOf(song.id to file),emptyMap())
                instrument.runOnMainSync {controller!!.pause();controller!!.volume=0f;controller!!.setMediaItem(song.media());controller!!.prepare();controller!!.play()}
                var decoded=false
                compose.waitUntil(10000){instrument.runOnMainSync{decoded=controller!!.isPlaying&&controller!!.currentPosition>100};decoded}
                instrument.runOnMainSync {assertNull(controller!!.playerError);assertEquals(Player.STATE_READY,controller!!.playbackState);assertTrue(controller!!.duration>=29000);controller!!.pause();controller!!.clearMediaItems()}
            }} finally {instrument.runOnMainSync {controller?.pause();controller?.clearMediaItems();controller?.release()}}
        }
    }
}
