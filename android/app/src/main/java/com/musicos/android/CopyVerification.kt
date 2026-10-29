package com.musicos.android

import android.system.Os
import java.io.File
import org.json.JSONObject

/** Receipts apply only to immutable objects in musicOS's private storage.
 * A changed size, timestamp, inode, or device requires a full checksum again. */
object CopyVerification {
    private fun stamp(file: File): String? {
        if (android.os.Build.VERSION.SDK_INT < 27) return null
        val stat = Os.stat(file.path)
        // Unavailable or synthetic stat data must never become a cache key.
        if (stat.st_ino <= 0 || stat.st_size != file.length() ||
            stat.st_mtim.tv_sec * 1000 + stat.st_mtim.tv_nsec / 1_000_000 != file.lastModified()) return null
        return "${stat.st_size}:${stat.st_dev}:${stat.st_ino}:${stat.st_mtim.tv_sec}:${stat.st_mtim.tv_nsec}:${stat.st_ctim.tv_sec}:${stat.st_ctim.tv_nsec}"
    }
    fun verified(file: File, hash: String, size: Long?): Boolean {
        if (!file.isFile || size != null && file.length() != size) return false
        val receipt = File(file.path + ".verified")
        val current = runCatching { stamp(file) }.getOrNull()
        if (current != null && runCatching {
            val value = JSONObject(receipt.readText())
            value.optString("hash") == hash && value.optString("stamp") == current
        }.getOrDefault(false)) return true
        val valid = WifiClient.verified(file, hash, size)
        if (valid && current == runCatching { stamp(file) }.getOrNull()) remember(file, hash)
        return valid
    }
    fun remember(file: File, hash: String) {
        runCatching {
            val signature = stamp(file) ?: return@runCatching
            val receipt = File(file.path + ".verified")
            val temporary = File(receipt.path + ".tmp-${Thread.currentThread().id}")
            temporary.writeText(JSONObject().put("hash", hash).put("stamp", signature).toString())
            if (!temporary.renameTo(receipt)) temporary.delete()
        }
    }
}
