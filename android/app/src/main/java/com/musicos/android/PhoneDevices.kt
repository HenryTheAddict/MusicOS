package com.musicos.android

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.work.WorkInfo
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions

@Composable
internal fun DeviceButton(label: String, enabled: Boolean = true, icon: String? = null, onClick: () -> Unit) {
    FilledTonalButton(onClick, enabled = enabled, shape = RoundedCornerShape(14.dp), contentPadding = PaddingValues(horizontal = 14.dp, vertical = 10.dp)) {
        if (icon != null) { SketchIcon(icon, Modifier.size(20.dp)); Spacer(Modifier.width(8.dp)) }
        Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
internal fun DevicesPage(model: PhoneModel, link: String, onLink: (String) -> Unit) {
    val macs by model.macs.collectAsStateWithLifecycle()
    val pairing by model.pairing.collectAsStateWithLifecycle()
    val status by model.pairingStatus.collectAsStateWithLifecycle()
    val library by model.library.collectAsStateWithLifecycle()
    val colors = LocalMusicColors.current
    val context = LocalContext.current
    var showPairing by remember { mutableStateOf(macs.isEmpty()) }
    var showLink by remember { mutableStateOf(link.isNotBlank()) }
    var pending by remember { mutableStateOf<(() -> Unit)?>(null) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        val action = pending; pending = null
        if (Build.VERSION.SDK_INT < 37 || androidx.core.content.ContextCompat.checkSelfPermission(context, "android.permission.ACCESS_LOCAL_NETWORK") == android.content.pm.PackageManager.PERMISSION_GRANTED) action?.invoke()
        else model.message.value = "Allow local network access in Android Settings to connect to your Mac."
    }
    fun network(action: () -> Unit) {
        val permissions = buildList {
            if (Build.VERSION.SDK_INT >= 37) add("android.permission.ACCESS_LOCAL_NETWORK")
            if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
        }.filter { androidx.core.content.ContextCompat.checkSelfPermission(context, it) != android.content.pm.PackageManager.PERMISSION_GRANTED }
        if (permissions.isEmpty()) action() else { pending = action; permission.launch(permissions.toTypedArray()) }
    }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { result ->
        result.contents?.let { value ->
            runCatching { PairLink.parse(value) }.onSuccess {
                showPairing = true; onLink(value); network { model.pair(value) }
            }.onFailure { model.message.value = "This isn’t a musicOS pairing code. Open Devices on your Mac and create a new link." }
        }
    }
    LaunchedEffect(link) { if (link.isNotBlank()) { showPairing = true; showLink = true } }
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 16.dp), contentPadding = PaddingValues(top = 16.dp, bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        item {
            Text("Devices", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
            Text("Your Mac. Your music. Anywhere.", color = colors.secondary, modifier = Modifier.padding(top = 4.dp))
        }
        item {
            Surface(color = colors.control, shape = RoundedCornerShape(20.dp)) {
                Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    SketchIcon("Phone", Modifier.size(32.dp))
                    Column(Modifier.weight(1f)) { Text("On this phone", fontWeight = FontWeight.Bold); Text("${songCount(library.songs.count { it.available })} ready offline", fontSize = 13.sp, color = colors.secondary) }
                    SketchIcon("Downloaded", Modifier.size(26.dp))
                }
            }
        }
        items(macs, key = { it.id }) { mac ->
            PairedMacCard(model, mac, library.songs.count { it.serverId == mac.id && it.available }) { action -> network(action) }
        }
        item {
            Surface(shape = RoundedCornerShape(20.dp), color = colors.surface, border = BorderStroke(1.dp, colors.ink.copy(alpha = .14f))) {
                Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) { Text(if(macs.isEmpty()) "Bring your Mac along" else "Another Mac?", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold); Text("Pair once on the same Wi-Fi.", color = colors.secondary, fontSize = 13.sp) }
                        if(macs.isNotEmpty()) DeviceButton(if(showPairing) "Close" else "Add", icon = if(showPairing) "Back" else "Plus") { showPairing = !showPairing }
                    }
                    AnimatedVisibility(showPairing || macs.isEmpty(), enter = expandVertically(tween(140)) + fadeIn(tween(140)), exit = shrinkVertically(tween(140)) + fadeOut(tween(140))) {
                        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text("1. On your Mac, open Devices and create a pairing link.\n2. Scan its code and approve the request on your Mac.\n3. Choose Copy music below. Keep listening as songs arrive.", fontSize = 13.sp, lineHeight = 20.sp, color = colors.secondary)
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                DeviceButton("Scan Mac code", !pairing, "device") { scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE).setPrompt("Scan the pairing code in musicOS on your Mac").setBeepEnabled(false).setOrientationLocked(false)) }
                                DeviceButton(if(showLink) "Hide link" else "Paste a link", !pairing) { showLink = !showLink }
                            }
                            if(showLink) {
                                OutlinedTextField(link, onLink, Modifier.fillMaxWidth(), label = { Text("Pairing link") }, enabled = !pairing, maxLines = 3, shape = RoundedCornerShape(14.dp))
                                DeviceButton(if(pairing) "Waiting for approval…" else "Connect to Mac", !pairing && link.isNotBlank(), "sync") { network { model.pair(link) } }
                            }
                            if(status.isNotBlank()) Text(status, color = colors.ink, fontSize = 13.sp)
                        }
                    }
                }
            }
        }
        item { Text("Your Mac chooses the music and copy quality. Only new or changed files are copied; finished songs and your phone’s favorites stay put if a transfer stops.", color = colors.secondary, fontSize = 12.sp, lineHeight = 18.sp) }
    }
}

@Composable
private fun PairedMacCard(model: PhoneModel, mac: PairedMac, offline: Int, network: (() -> Unit) -> Unit) {
    val colors = LocalMusicColors.current
    val jobs by model.jobs.collectAsStateWithLifecycle()
    val id = jobs[mac.id]
    val info = if(id == null) null else model.work.getWorkInfoByIdFlow(id).collectAsStateWithLifecycle(initialValue = null).value
    val active = info != null && !info.state.isFinished
    val done = info?.progress?.getLong("done", 0) ?: 0
    val total = info?.progress?.getLong("total", 0) ?: 0
    val count = info?.progress?.getInt("count", 0) ?: 0
    val songs = info?.progress?.getInt("songs", 0) ?: 0
    var details by remember(mac.id) { mutableStateOf(false) }
    Surface(shape = RoundedCornerShape(20.dp), color = colors.control.copy(alpha = .5f), border = BorderStroke(1.dp, colors.ink.copy(alpha = .12f))) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                SketchIcon("device", Modifier.size(28.dp))
                Column(Modifier.weight(1f)) { Text(mac.name, fontWeight = FontWeight.Bold, fontSize = 20.sp, maxLines = 2); Text("Paired · ${songCount(offline)} on this phone", color = colors.secondary, fontSize = 12.sp) }
            }
            if(info != null) {
                Text(when(info.state) {
                    WorkInfo.State.SUCCEEDED -> "Up to date · ready offline"
                    WorkInfo.State.CANCELLED -> "Copy stopped. Finished songs are ready to play."
                    WorkInfo.State.FAILED -> info.outputData.getString("error") ?: "Couldn’t connect. Check that the Mac is sharing on the same Wi-Fi."
                    WorkInfo.State.ENQUEUED -> "Connecting when Wi-Fi and storage are ready…"
                    else -> info.progress.getString("message") ?: "Connecting to your Mac…"
                }, color = colors.ink, fontSize = 13.sp, lineHeight = 18.sp)
            }
            if(active) {
                if(total > 0) LinearProgressIndicator(progress = { (done.toFloat()/total).coerceIn(0f,1f) }, modifier = Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.ink.copy(alpha = .12f)) else LinearProgressIndicator(Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.ink.copy(alpha = .12f))
                if(songs > 0) Text("$count of $songs songs · ${(done/1048576)} of ${(total/1048576)} MB", fontSize = 12.sp, color = colors.secondary)
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                DeviceButton(if(active) "Copying…" else if(info?.state == WorkInfo.State.FAILED) "Retry copy" else if(offline > 0) "Check for new music" else "Copy music", !active, if(offline > 0) "sync" else "Download") { network { model.sync(mac) } }
                if(active) DeviceButton("Stop copying") { model.cancel(mac) } else DeviceButton(if(details) "Less" else "Details") { details = !details }
            }
            if(details && !active) { Text("Connect to the same Wi-Fi and leave sharing on in musicOS for Mac. Your offline library stays playable without the Mac.", fontSize = 12.sp, color = colors.secondary); DeviceButton("Forget this Mac") { model.forget(mac) } }
        }
    }
}

@Composable
internal fun MusicMenu(expanded: Boolean, dismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    val colors = LocalMusicColors.current
    DropdownMenu(expanded, dismiss, shape = RoundedCornerShape(16.dp), containerColor = colors.surface, tonalElevation = 0.dp, shadowElevation = 3.dp, border = BorderStroke(1.dp, colors.ink.copy(alpha = .14f)), content = content)
}
