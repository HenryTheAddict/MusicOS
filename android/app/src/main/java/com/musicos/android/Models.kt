package com.musicos.android

import java.net.URI
import java.security.MessageDigest
import java.security.SecureRandom
import org.json.JSONArray
import org.json.JSONObject

fun capability(): String =
    ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }

fun sha256(bytes: ByteArray): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

fun String.requireHash(): String {
    require(matches(Regex("[a-fA-F0-9]{64}"))) { "Invalid sync fingerprint" }
    return lowercase()
}

data class PairLink(
    val host: String,
    val port: Int,
    val server: String,
    val fingerprint: String,
    val ticket: String,
) {
    companion object {
        fun parse(text: String): PairLink {
            val uri = URI(text.trim())
            require(uri.scheme == "musicos" && uri.host == "pair" && uri.rawUserInfo == null) {
                "Paste the pairing link shown by MusicOS on your Mac"
            }
            val parts =
                uri.rawQuery.orEmpty().split("&").associate {
                    val p = it.split("=", limit = 2)
                    require(p.size == 2)
                    p[0] to java.net.URLDecoder.decode(p[1], "UTF-8")
                }
            val host = parts["host"].orEmpty()
            require(isLocalAddress(host)) { "Use a local Wi-Fi address from the Mac" }
            val port = parts["port"]?.toIntOrNull() ?: 0
            require(port in 1024..65535) { "Invalid sync port" }
            return PairLink(
                host,
                port,
                parts.getValue("server").requireHash(),
                parts.getValue("fingerprint").requireHash(),
                parts.getValue("ticket").requireHash(),
            )
        }
    }
}

fun isLocalAddress(host: String): Boolean {
    val parts = host.split(".").map { it.toIntOrNull() ?: return false }
    if (parts.size != 4 || parts.any { it !in 0..255 }) return false
    return parts[0] == 10 ||
        parts[0] == 127 ||
        parts[0] == 192 && parts[1] == 168 ||
        parts[0] == 172 && parts[1] in 16..31 ||
        parts[0] == 169 && parts[1] == 254
}

data class PairedMac(
    val id: String,
    val name: String,
    val host: String,
    val port: Int,
    val fingerprint: String,
    val clientId: String,
    val token: String,
) {
    fun json() =
        JSONObject()
            .put("id", id)
            .put("name", name)
            .put("host", host)
            .put("port", port)
            .put("fingerprint", fingerprint)
            .put("client_id", clientId)
            .put("token", token)

    companion object {
        fun parse(j: JSONObject) =
            PairedMac(
                j.getString("id").requireHash(),
                j.getString("name"),
                j.getString("host").also { require(isLocalAddress(it)) },
                j.getInt("port").also { require(it in 1024..65535) },
                j.getString("fingerprint").requireHash(),
                j.getString("client_id").requireHash(),
                j.getString("token").requireHash(),
            )
    }
}

data class Song(
    val id: String,
    val serverId: String,
    val remoteId: Long,
    val title: String,
    val artist: String,
    val album: String,
    val albumId: String,
    val genre: String,
    val year: Int,
    val trackNumber: Int,
    val discNumber: Int,
    val duration: Long,
    val size: Long,
    val hash: String,
    val mime: String,
    val artwork: String?,
    val favorite: Boolean,
    val localFile: String?,
    val artworkFile: String?,
) {
    val available: Boolean
        get() = localFile != null
}

data class MusicCollection(
    val id: String,
    val serverId: String,
    val name: String,
    val kind: String,
    val songIds: List<String>,
)

data class MusicLibrary(
    val songs: List<Song> = emptyList(),
    val collections: List<MusicCollection> = emptyList(),
    val displayName: String = "",
    val ready: Boolean = false,
)

data class SyncManifest(
    val serverId: String,
    val name: String,
    val displayName: String,
    val revision: String,
    val tracks: List<Song>,
    val collections: List<MusicCollection>,
    val bytes: Long,
) {
    companion object {
        fun parse(j: JSONObject, expectedServer: String): SyncManifest {
            require(j.getInt("version") == 1 && j.getString("server_id") == expectedServer) {
                "This is a different MusicOS library. Pair it separately."
            }
            val rows = j.getJSONArray("tracks")
            require(rows.length() <= 50000)
            val ids = mutableSetOf<Long>()
            val tracks =
                (0 until rows.length()).map { index ->
                    val row = rows.getJSONObject(index)
                    val id = row.getLong("id")
                    require(id > 0 && ids.add(id)) { "Duplicate song identity" }
                    val size = row.getLong("size")
                    require(size in 1..(20L * 1024 * 1024 * 1024)) { "Unsupported song size" }
                    Song(
                        "$expectedServer/$id",
                        expectedServer,
                        id,
                        row.getString("title").take(2000),
                        row.getString("artist").take(2000),
                        row.getString("album").take(2000),
                        "$expectedServer/${row.getString("album_id")}",
                        row.optString("genre"),
                        row.optInt("year"),
                        row.optInt("track_number"),
                        row.optInt("disc_number"),
                        row.getLong("duration_ms").coerceAtLeast(0),
                        size,
                        row.getString("sha256").requireHash(),
                        row.getString("mime"),
                        if (row.isNull("artwork_sha256")) null
                        else row.getString("artwork_sha256").requireHash(),
                        row.optBoolean("favorited"),
                        null,
                        null,
                    )
                }
            val collections = j.optJSONArray("collections") ?: JSONArray()
            require(collections.length() <= 10000)
            val result =
                (0 until collections.length()).map { index ->
                    val row = collections.getJSONObject(index)
                    val sequence = row.getJSONArray("track_ids")
                    require(sequence.length() <= 50000)
                    MusicCollection(
                        "$expectedServer/${row.getString("id")}",
                        expectedServer,
                        row.getString("name").take(2000),
                        row.getString("kind"),
                        (0 until sequence.length()).map {
                            sequence
                                .getLong(it)
                                .also { id -> require(id in ids) }
                                .let { id -> "$expectedServer/$id" }
                        },
                    )
                }
            val bytes = tracks.fold(0L) { total, song -> Math.addExact(total, song.size) }
            require(bytes == j.getLong("bytes_total")) { "Invalid transfer size" }
            return SyncManifest(
                expectedServer,
                j.getString("server_name"),
                j.optString("display_name"),
                j.getString("revision").requireHash(),
                tracks,
                result,
                bytes,
            )
        }
    }
}
