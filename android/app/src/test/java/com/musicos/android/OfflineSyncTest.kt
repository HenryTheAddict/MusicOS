package com.musicos.android

import androidx.test.core.app.ApplicationProvider
import java.io.File
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.tls.HandshakeCertificates
import okhttp3.tls.HeldCertificate
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = MusicOSApp::class)
class OfflineSyncTest {
    private val identity = "a".repeat(64)

    private fun manifest(): JSONObject {
        val song =
            JSONObject()
                .put("id", 7)
                .put("title", "Offline song")
                .put("artist", "musicOS")
                .put("album", "Fixture")
                .put("album_id", "album")
                .put("duration_ms", 1000)
                .put("size", 5)
                .put("sha256", sha256("music".toByteArray()))
                .put("mime", "audio/flac")
                .put("artwork_sha256", JSONObject.NULL)
                .put("favorited", true)
        return JSONObject()
            .put("version", 1)
            .put("server_id", identity)
            .put("server_name", "Test Mac")
            .put("display_name", "Henry")
            .put("revision", "b".repeat(64))
            .put("tracks", JSONArray().put(song))
            .put(
                "collections",
                JSONArray()
                    .put(
                        JSONObject()
                            .put("id", "list")
                            .put("name", "Repeated")
                            .put("kind", "manual")
                            .put("track_ids", JSONArray().put(7).put(7))
                    ),
            )
            .put("bytes_total", 5)
    }

    @Test
    fun pairingOnlyAcceptsPrivateAddressesAndFullFingerprints() {
        val valid =
            "musicos://pair?host=192.168.1.2&port=4000&server=$identity&fingerprint=$identity&ticket=$identity"
        assertEquals(4000, PairLink.parse(valid).port)
        for (link in
            listOf(
                valid.replace("192.168.1.2", "8.8.8.8"),
                valid.replace("4000", "22"),
                valid.replace("fingerprint=$identity", "fingerprint=abc"),
                valid.replace("musicos://", "https://"),
            )) {
            assertTrue(runCatching { PairLink.parse(link) }.isFailure)
        }
        assertNotEquals(capability(), capability())
    }

    @Test
    fun manifestsPreserveRepeatedSequenceAndRejectForeignOrDuplicateSongs() {
        val parsed = SyncManifest.parse(manifest(), identity)
        assertEquals(listOf("$identity/7", "$identity/7"), parsed.collections.single().songIds)
        assertEquals(5, parsed.bytes)
        val duplicate = manifest()
        duplicate.getJSONArray("tracks").put(duplicate.getJSONArray("tracks").getJSONObject(0))
        assertTrue(runCatching { SyncManifest.parse(duplicate, identity) }.isFailure)
        assertTrue(runCatching { SyncManifest.parse(manifest(), "c".repeat(64)) }.isFailure)
        val bad = manifest().put("bytes_total", 99)
        assertTrue(runCatching { SyncManifest.parse(bad, identity) }.isFailure)
    }

    @Test
    fun importIsAtomicAndRetainsPhoneFavorites() {
        val context = ApplicationProvider.getApplicationContext<MusicOSApp>()
        context.deleteDatabase("musicos-offline-v1.db")
        val repo = MusicRepository(context)
        val parsed = SyncManifest.parse(manifest(), identity)
        assertTrue(runCatching { repo.import(parsed, emptyMap(), emptyMap()) }.isFailure)
        repo.refresh()
        assertTrue(repo.snapshots.value.songs.isEmpty())
        val file = repo.objectFile(parsed.tracks.single().hash)
        file.writeText("music")
        repo.import(parsed, mapOf("$identity/7" to file), emptyMap())
        assertEquals(2, repo.snapshots.value.collections.single().songIds.size)
        repo.writableDatabase.execSQL(
            "UPDATE songs SET favorite=0,favorite_dirty=1 WHERE id=?",
            arrayOf("$identity/7"),
        )
        repo.import(parsed, mapOf("$identity/7" to file), emptyMap())
        assertFalse(repo.snapshots.value.songs.single().favorite)
        assertEquals("Henry", repo.snapshots.value.displayName)
        repo.close()
    }

    private fun tlsServer(): Pair<MockWebServer, PairedMac> {
        val certificate =
            HeldCertificate.Builder()
                .commonName("localhost")
                .addSubjectAlternativeName("localhost")
                .build()
        val tls = HandshakeCertificates.Builder().heldCertificate(certificate).build()
        val server = MockWebServer()
        server.useHttps(tls.sslSocketFactory(), false)
        server.start()
        return server to
            PairedMac(
                identity,
                "Test Mac",
                "127.0.0.1",
                server.port,
                sha256(certificate.certificate.encoded),
                "c".repeat(64),
                "d".repeat(64),
            )
    }

    @Test
    fun preparationWaitsInTheActiveTransferAndPreservesTheVorbisMime() = runBlocking<Unit> {
        val (server, mac) = tlsServer()
        try {
            server.enqueue(MockResponse().setResponseCode(503))
            server.enqueue(MockResponse().setBody("{\"preparing\":true,\"completed\":42,\"total\":508}"))
            val ready = manifest()
            ready.getJSONArray("tracks").getJSONObject(0).put("mime", "audio/ogg")
            server.enqueue(MockResponse().setBody(ready.toString()))
            val messages = mutableListOf<String>()
            val result = WifiClient(mac).manifest { messages.add(it) }
            assertEquals("audio/ogg", result.tracks.single().mime)
            assertEquals(listOf("Mac is preparing music · 42 of 508 songs"), messages)
            assertEquals("/v1/manifest", server.takeRequest().path)
            assertEquals("/v1/preparation", server.takeRequest().path)
            assertEquals("/v1/manifest", server.takeRequest().path)
        } finally { server.shutdown() }
    }

    @Test
    fun pinnedDownloadResumesAndVerifiesBeforePublishing() =
        runBlocking<Unit> {
            val (server, mac) = tlsServer()
            try {
                val folder = java.nio.file.Files.createTempDirectory("musicos-test").toFile()
                val target = File(folder, "verified.audio")
                File(target.path + ".partial").writeText("mu")
                server.enqueue(
                    MockResponse()
                        .setResponseCode(206)
                        .setHeader("Content-Range", "bytes 2-4/5")
                        .setBody("sic")
                )
                WifiClient(mac).download("tracks/7", target, sha256("music".toByteArray()), 5) {}
                assertEquals("music", target.readText())
                val request = server.takeRequest()
                assertEquals("bytes=2-", request.getHeader("Range"))
                assertEquals("\"${sha256("music".toByteArray())}\"", request.getHeader("If-Match"))
                assertEquals(
                    "Bearer ${mac.clientId}.${mac.token}",
                    request.getHeader("Authorization"),
                )
                assertFalse(request.path!!.contains(mac.token))
                folder.deleteRecursively()
            } finally {
                server.shutdown()
            }
        }

    @Test
    fun hashFailureNeverPublishesAndWrongCertificateIsRejected() =
        runBlocking<Unit> {
            val (server, mac) = tlsServer()
            try {
                val folder = java.nio.file.Files.createTempDirectory("musicos-test").toFile()
                val target = File(folder, "verified.audio")
                server.enqueue(MockResponse().setBody("wrong"))
                assertTrue(
                    runCatching {
                            WifiClient(mac).download(
                                "tracks/7",
                                target,
                                sha256("music".toByteArray()),
                                5,
                            ) {}
                        }
                        .isFailure
                )
                assertFalse(target.exists())
                assertFalse(File(target.path + ".partial").exists())
                server.enqueue(MockResponse().setBody("{}"))
                assertTrue(
                    runCatching { WifiClient(mac.copy(fingerprint = "0".repeat(64))).json("info") }
                        .isFailure
                )
                folder.deleteRecursively()
            } finally {
                server.shutdown()
            }
        }
}
