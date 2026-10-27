package com.musicos.android

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull

data class DiscoveredMac(
    val id: String,
    val name: String,
    val host: String,
    val port: Int,
    val fingerprint: String,
)

/** Discovery only locates a pinned identity; advertisements never authorize a phone. */
class MacDiscovery(context: Context) {
    private val nsd = context.getSystemService(NsdManager::class.java)

    @Suppress("DEPRECATION")
    suspend fun find(id: String, fingerprint: String): DiscoveredMac? =
        withTimeoutOrNull(5000) {
            suspendCancellableCoroutine { continuation ->
                lateinit var listener: NsdManager.DiscoveryListener
                fun stop() {
                    runCatching { nsd.stopServiceDiscovery(listener) }
                }
                listener =
                    object : NsdManager.DiscoveryListener {
                        override fun onDiscoveryStarted(type: String) {}

                        override fun onDiscoveryStopped(type: String) {}

                        override fun onStartDiscoveryFailed(type: String, error: Int) {
                            if (continuation.isActive) continuation.resume(null)
                            stop()
                        }

                        override fun onStopDiscoveryFailed(type: String, error: Int) {}

                        override fun onServiceLost(service: NsdServiceInfo) {}

                        override fun onServiceFound(service: NsdServiceInfo) {
                            if (service.serviceType != "_musicos-sync._tcp.") return
                            nsd.resolveService(
                                service,
                                object : NsdManager.ResolveListener {
                                    override fun onResolveFailed(
                                        info: NsdServiceInfo,
                                        error: Int,
                                    ) {}

                                    override fun onServiceResolved(info: NsdServiceInfo) {
                                        val identity =
                                            info.attributes["id"]?.toString(Charsets.UTF_8)
                                        val pin =
                                            info.attributes["fingerprint"]?.toString(Charsets.UTF_8)
                                        val address = info.host?.hostAddress ?: return
                                        if (
                                            identity == id &&
                                                pin == fingerprint &&
                                                isLocalAddress(address) &&
                                                continuation.isActive
                                        ) {
                                            continuation.resume(
                                                DiscoveredMac(
                                                    id,
                                                    info.serviceName,
                                                    address,
                                                    info.port,
                                                    fingerprint,
                                                )
                                            )
                                            stop()
                                        }
                                    }
                                },
                            )
                        }
                    }
                continuation.invokeOnCancellation { stop() }
                try {
                    nsd.discoverServices(
                        "_musicos-sync._tcp.",
                        NsdManager.PROTOCOL_DNS_SD,
                        listener,
                    )
                } catch (_: Exception) {
                    if (continuation.isActive) continuation.resume(null)
                }
            }
        }
}
