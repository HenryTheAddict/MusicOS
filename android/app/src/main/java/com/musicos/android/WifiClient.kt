package com.musicos.android

import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
import java.security.MessageDigest
import java.security.cert.X509Certificate
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLContext
import javax.net.ssl.X509TrustManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

class SyncHttpError(val code: Int, message: String) : IOException(message)

class WifiClient(private val mac: PairedMac) {
    private val trust =
        object : X509TrustManager {
            override fun getAcceptedIssuers() = emptyArray<X509Certificate>()

            override fun checkClientTrusted(chain: Array<X509Certificate>, authType: String) {
                throw java.security.cert.CertificateException("Client certificates are not used")
            }

            override fun checkServerTrusted(chain: Array<X509Certificate>, authType: String) {
                if (
                    chain.isEmpty() ||
                        !MessageDigest.isEqual(
                            sha256(chain[0].encoded).toByteArray(),
                            mac.fingerprint.toByteArray(),
                        )
                )
                    throw java.security.cert.CertificateException(
                        "This Mac's fingerprint changed. Pair again from Devices."
                    )
                chain[0].checkValidity()
            }
        }
    private val ssl = SSLContext.getInstance("TLS").apply { init(null, arrayOf(trust), null) }
    private val http =
        OkHttpClient.Builder()
            .sslSocketFactory(ssl.socketFactory, trust)
            .hostnameVerifier { _, session ->
                runCatching { sha256(session.peerCertificates[0].encoded) == mac.fingerprint }
                    .getOrDefault(false)
            }
            .followRedirects(false)
            .followSslRedirects(false)
            .connectTimeout(6, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .build()

    private fun request(path: String) =
        Request.Builder()
            .url("https://${mac.host}:${mac.port}/v1/$path")
            .header("Authorization", "Bearer ${mac.clientId}.${mac.token}")

    private fun error(code: Int): String =
        when (code) {
            401,
            403 -> "This phone is no longer authorized. Pair it again in Devices."
            409,
            412 -> "The music selection or copy format changed. Retry Copy music on the phone."
            429 -> "Too many pairing attempts. Try again in a minute."
            else -> "The Mac could not finish the transfer (HTTP $code)."
        }

    suspend fun json(path: String, body: JSONObject? = null): JSONObject =
        withContext(Dispatchers.IO) {
            val req =
                request(path)
                    .apply {
                        if (body != null)
                            post(body.toString().toRequestBody("application/json".toMediaType()))
                    }
                    .build()
            http.newCall(req).execute().use { response ->
                if (!response.isSuccessful) throw SyncHttpError(response.code, error(response.code))
                val stream =
                    response.body?.byteStream() ?: throw IOException("Empty response from Mac")
                val out = java.io.ByteArrayOutputStream()
                val buffer = ByteArray(65536)
                while (true) {
                    currentCoroutineContext().ensureActive()
                    val n = stream.read(buffer)
                    if (n < 0) break
                    require(out.size() + n <= 16 * 1024 * 1024) { "Library response is too large" }
                    out.write(buffer, 0, n)
                }
                JSONObject(out.toString("UTF-8"))
            }
        }

    suspend fun manifest(waiting: suspend (String) -> Unit = {}): SyncManifest {
        while (true) {
            currentCoroutineContext().ensureActive()
            try { return SyncManifest.parse(json("manifest"), mac.id) }
            catch (error: SyncHttpError) {
                if (error.code != 503) throw error
                val progress = json("preparation")
                val done = progress.optInt("completed")
                val total = progress.optInt("total")
                waiting("Mac is preparing music · $done of $total songs")
                delay(1000)
            }
        }
    }

    suspend fun pair(ticket: String, name: String) =
        json(
            "pair",
            JSONObject()
                .put("ticket", ticket)
                .put("client_id", mac.clientId)
                .put("client_name", name)
                .put("client_token", mac.token),
        )

    suspend fun pairStatus() = json("pair/status")

    suspend fun complete(manifest: SyncManifest) =
        json(
            "sync-complete",
            JSONObject()
                .put("revision", manifest.revision)
                .put("track_count", manifest.tracks.size)
                .put("bytes", manifest.bytes),
        )

    suspend fun download(
        path: String,
        target: File,
        expectedHash: String,
        expectedSize: Long?,
        progress: suspend (Long) -> Unit,
    ): File =
        withContext(Dispatchers.IO) {
            if (target.isFile && CopyVerification.verified(target, expectedHash, expectedSize))
                return@withContext target
            val partial = File(target.path + ".partial")
            if (expectedSize != null && partial.length() > expectedSize) partial.delete()
            val offset = partial.length()
            val call =
                http.newCall(
                    request(path)
                        .header("If-Match", "\"$expectedHash\"")
                        .apply { if (offset > 0) header("Range", "bytes=$offset-") }
                        .build()
                )
            call.execute().use { response ->
                if (!response.isSuccessful) {
                    if (response.code == 416) partial.delete()
                    throw SyncHttpError(response.code, error(response.code))
                }
                val append = offset > 0 && response.code == 206
                if (
                    append && response.header("Content-Range")?.startsWith("bytes $offset-") != true
                )
                    throw IOException("Invalid resume response. Retry this transfer.")
                val body = response.body ?: throw IOException("Empty music response")
                RandomAccessFile(partial, "rw").use { out ->
                    if (append) out.seek(offset) else out.setLength(0)
                    body.byteStream().use { input ->
                        val buffer = ByteArray(65536)
                        var written = if (append) offset else 0L
                        while (true) {
                            currentCoroutineContext().ensureActive()
                            val n = input.read(buffer)
                            if (n < 0) break
                            written += n
                            val maximum = expectedSize ?: 5L * 1024 * 1024
                            if (written > maximum)
                                throw IOException(
                                    "Transfer size changed. Refresh the Mac selection."
                                )
                            out.write(buffer, 0, n)
                            progress(written)
                        }
                        out.fd.sync()
                    }
                }
            }
            currentCoroutineContext().ensureActive()
            if (!verified(partial, expectedHash, expectedSize)) {
                partial.delete()
                throw IOException(
                    "File verification failed. Retry while connected to the same Wi-Fi."
                )
            }
            if (!partial.renameTo(target))
                throw IOException(
                    "Could not save offline music; the verified download was retained"
                )
            CopyVerification.remember(target, expectedHash)
            target
        }

    companion object {
        fun verified(file: File, hash: String, size: Long?): Boolean {
            if (!file.isFile || size != null && file.length() != size) return false
            val digest = MessageDigest.getInstance("SHA-256")
            file.inputStream().use { input ->
                val buffer = ByteArray(65536)
                while (true) {
                    val n = input.read(buffer)
                    if (n < 0) break
                    digest.update(buffer, 0, n)
                }
            }
            return MessageDigest.isEqual(
                digest.digest().joinToString("") { "%02x".format(it) }.toByteArray(),
                hash.toByteArray(),
            )
        }
    }
}
