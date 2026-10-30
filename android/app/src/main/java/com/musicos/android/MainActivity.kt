package com.musicos.android

import android.Manifest
import android.content.Intent
import android.media.audiofx.AudioEffect
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.media3.common.Player
import androidx.media3.session.SessionCommand
import androidx.work.WorkInfo
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private val incoming = mutableStateOf("")
    private val voice = mutableStateOf("")

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        enableEdgeToEdge()
        readIntent(intent)
        setContent { MusicApp(incoming.value, voice.value) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        readIntent(intent)
    }

    private fun readIntent(value: Intent?) {
        incoming.value = if(value?.getBooleanExtra("open_devices", false)==true) "musicos://devices" else value?.data?.toString().orEmpty()
        voice.value =
            if (value?.action == android.provider.MediaStore.INTENT_ACTION_MEDIA_PLAY_FROM_SEARCH)
                value.getStringExtra(android.app.SearchManager.QUERY).orEmpty()
            else ""
    }
}

internal fun songCount(count: Int) = "$count ${if(count == 1) "song" else "songs"}"

private fun time(ms: Long) = "${ms/60000}:${(ms/1000%60).toString().padStart(2,'0')}"

@Composable
private fun MusicButton(label: String, icon: String? = null, onClick: () -> Unit) {
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by
        animateFloatAsState(if (pressed) .98f else 1f, tween(140), label = "Soft button press")
    FilledTonalButton(
        onClick,
        modifier =
            Modifier.graphicsLayer {
                scaleX = scale
                scaleY = scale
            },
        interactionSource = interaction,
        shape = RoundedCornerShape(14.dp),
        contentPadding = PaddingValues(horizontal = 14.dp, vertical = 10.dp),
    ) {
        if (icon != null) {
            SketchIcon(icon, Modifier.size(20.dp))
            Spacer(Modifier.width(8.dp))
        }
        Text(label)
    }
}

@Composable
private fun IconAction(label: String, icon: String, onClick: () -> Unit) {
    IconButton(onClick, Modifier.size(48.dp).semantics { contentDescription = label }) {
        SketchIcon(icon)
    }
}

@Composable
private fun Transport(model: PhoneModel, state: Playing, large: Boolean = false) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
        modifier = Modifier.fillMaxWidth(),
    ) {
        IconButton(model::shuffle, Modifier.size(48.dp).semantics { contentDescription = "Shuffle" }) {
            SketchIcon(if (state.shuffle) "ShuffleON" else "ShuffleOFF", Modifier.size(22.dp),
                if (state.shuffle) LocalMusicColors.current.accent else LocalMusicColors.current.secondary)
        }
        IconAction("Previous", "Previous", model::previous)
        IconButton(
            model::toggle,
            Modifier.size(if (large) 64.dp else 48.dp).semantics {
                contentDescription = if (state.playing) "Pause" else "Play"
            },
        ) {
            PlayPauseIcon(state.playing, Modifier.size(if (large) 44.dp else 32.dp))
        }
        IconAction("Next", "Next", model::next)
        IconButton(model::repeat, Modifier.size(48.dp).semantics {
            contentDescription = when (state.repeat) {
                Player.REPEAT_MODE_ONE -> "Repeat one"
                Player.REPEAT_MODE_ALL -> "Repeat all"
                else -> "Repeat off"
            }
        }) {
            SketchIcon(when (state.repeat) {
                Player.REPEAT_MODE_ONE -> "RepeatONE"
                Player.REPEAT_MODE_ALL -> "RepeatON"
                else -> "RepeatOFF"
            }, Modifier.size(22.dp), LocalMusicColors.current.secondary)
        }
    }
}

@Composable
private fun SongRow(song: Song, onPlay: () -> Unit, model: PhoneModel, index: Int? = null) {
    var menu by remember { mutableStateOf(false) }
    val colors = LocalMusicColors.current
    Row(
        Modifier.fillMaxWidth()
            .padding(vertical = 3.dp)
            .clip(RoundedCornerShape(18.dp))
            .background(colors.control.copy(alpha = .45f))
            .clickable(onClick = onPlay)
            .padding(start = 12.dp, end = 4.dp, top = 8.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (index != null)
            Text("${index+1}", Modifier.width(28.dp), style = MaterialTheme.typography.bodySmall)
        else Artwork(song.artworkFile, Modifier.size(44.dp))
        Column(Modifier.weight(1f).padding(horizontal = 10.dp)) {
            Text(
                song.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                fontWeight = FontWeight.SemiBold,
            )
            Text(
                song.artist,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                fontSize = 12.sp,
                color = colors.secondary,
            )
        }
        Text(time(song.duration), fontSize = 11.sp, color = colors.secondary)
        Box {
            IconButton({ menu = true }) { SketchIcon("More", Modifier.size(22.dp)) }
            MusicMenu(menu, { menu = false }) {
                DropdownMenuItem(
                    { Text(if (song.favorite) "Remove favorite" else "Favorite") },
                    {
                        menu = false
                        model.repository.favorite(song.id)
                    },
                )
                DropdownMenuItem(
                    { Text("Play next") },
                    {
                        menu = false
                        model.enqueue(song, true)
                    },
                )
                DropdownMenuItem(
                    { Text("Add to queue") },
                    {
                        menu = false
                        model.enqueue(song)
                    },
                )
            }
        }
    }
}

@Composable
private fun MusicApp(incoming: String, voice: String = "", model: PhoneModel = viewModel()) {
    val context = LocalContext.current
    val preferences = remember { context.getSharedPreferences("appearance-v1", 0) }
    val product=remember { PhoneProduct.load(context) }
    var welcome by rememberSaveable { mutableStateOf(false) }
    var welcomeVersion by remember { mutableIntStateOf(preferences.getInt("onboarding_version",0)) }
    val legacySetup=remember { preferences.contains("theme")||preferences.contains("colors") }
    var theme by rememberSaveable {
        mutableStateOf(preferences.getString("theme", product.mode) ?: product.mode)
    }
    val dark = theme == "dark" || theme == "system" && isSystemInDarkTheme()
    SideEffect {
        (context as? ComponentActivity)?.let { activity ->
            androidx.core.view.WindowCompat.getInsetsController(
                    activity.window,
                    activity.window.decorView,
                )
                .apply {
                    isAppearanceLightStatusBars = !dark
                    isAppearanceLightNavigationBars = !dark
                }
        }
    }
    var colorSource by rememberSaveable { mutableStateOf(preferences.getString("colors", product.colors) ?: product.colors) }
    val playing by model.playing.collectAsStateWithLifecycle()
    MusicTheme(dark, playing.song?.artworkFile?.takeIf { colorSource == "artwork" }) {
        val colors = LocalMusicColors.current
        val library by model.library.collectAsStateWithLifecycle()
        val pairingsReady by model.pairingsReady.collectAsStateWithLifecycle()
        val macs by model.macs.collectAsStateWithLifecycle()
        val existingSetup=legacySetup||library.songs.isNotEmpty()||macs.isNotEmpty()||incoming.isNotBlank()||voice.isNotBlank()
        LaunchedEffect(library.ready,pairingsReady,existingSetup) {if(library.ready&&pairingsReady&&existingSetup&&welcomeVersion<product.onboardingVersion){preferences.edit().putInt("onboarding_version",product.onboardingVersion).apply();welcomeVersion=product.onboardingVersion}}
        val message by model.message.collectAsStateWithLifecycle()
        var destination by rememberSaveable { mutableStateOf("Home") }
        var tools by remember { mutableStateOf(false) }
        var player by rememberSaveable { mutableStateOf(false) }
        var queue by rememberSaveable { mutableStateOf(false) }
        var query by rememberSaveable { mutableStateOf("") }
        var category by rememberSaveable { mutableStateOf("Albums") }
        var link by remember { mutableStateOf("") }
        var previousDestination by remember { mutableStateOf(destination) }
        LaunchedEffect(destination) {
            if (previousDestination == "Devices" && destination != "Devices") link = ""
            previousDestination = destination
        }
        var homeDetail by rememberSaveable { mutableStateOf<String?>(null) }
        var searchDetail by rememberSaveable { mutableStateOf<String?>(null) }
        var libraryDetail by rememberSaveable { mutableStateOf<String?>(null) }
        val detail =
            when (destination) {
                "Home" -> homeDetail
                "Search" -> searchDetail
                else -> libraryDetail
            }
        fun setDetail(value: String?) {
            when (destination) {
                "Home" -> homeDetail = value
                "Search" -> searchDetail = value
                else -> libraryDetail = value
            }
        }
        LaunchedEffect(incoming) {
            if (incoming == "musicos://devices") { destination = "Devices"; player = false }
            if (incoming.startsWith("musicos://pair")) {
                link = incoming
                destination = "Devices"
                player = false
            }
        }
        LaunchedEffect(voice) {
            if (voice.isNotBlank()) {
                val loaded = model.library.first { it.ready }
                val songs =
                    loaded.songs.filter {
                        "${it.title} ${it.artist} ${it.album}".contains(voice, ignoreCase = true)
                    }
                if (songs.isNotEmpty()) model.play(songs)
                else model.message.value = "No offline music matches this request."
            }
        }
        val owner = LocalLifecycleOwner.current
        LaunchedEffect(owner, playing.playing) {
            if (playing.playing)
                owner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
                    while (true) {
                        model.position()
                        delay(250)
                    }
                }
        }
        BackHandler(
            player || detail != null || destination == "Settings" || destination == "Devices"
        ) {
            when {
                player -> player = false
                detail != null -> setDetail(null)
                else -> destination = "Home"
            }
        }
        val homeScroll = rememberLazyListState()
        val libraryScroll = rememberLazyListState()
        val searchScroll = rememberLazyListState()
        val detailScroll = rememberLazyListState()
        Surface(Modifier.fillMaxSize(), color = colors.shell, contentColor = colors.ink) {
            if(!library.ready||!pairingsReady) {Box(Modifier.fillMaxSize(),contentAlignment=Alignment.Center){SketchIcon("Album",Modifier.size(48.dp))}} else if(welcome || library.ready&&pairingsReady&&!existingSetup&&welcomeVersion<product.onboardingVersion) {
                PhoneOnboarding(theme,colorSource,revisiting=welcome) { selectedMode,selectedColors,pair ->
                    val saved=kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {preferences.edit().putString("theme",selectedMode).putString("colors",selectedColors).putInt("onboarding_version",product.onboardingVersion).commit()}
                    if(saved){theme=selectedMode;colorSource=selectedColors;welcomeVersion=product.onboardingVersion;destination=if(pair)"Devices" else if(welcome)"Settings" else "Home";welcome=false};saved
                }
            } else Column(Modifier.fillMaxSize().safeDrawingPadding().padding(horizontal = 8.dp)) {
                if (!player) Row(
                    Modifier.fillMaxWidth().height(48.dp).padding(horizontal = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    SketchIcon("Album", Modifier.size(26.dp))
                    Text(
                        product.name,
                        Modifier.weight(1f).padding(start = 8.dp),
                        fontWeight = FontWeight.Bold,
                        fontSize = 18.sp,
                    )
                    Box {
                        FilledTonalButton({ tools = true }, shape = RoundedCornerShape(14.dp), contentPadding = PaddingValues(horizontal = 12.dp, vertical = 8.dp)) {
                            Text(destination, color = colors.ink)
                            Spacer(Modifier.width(8.dp))
                            val arrow by
                                animateFloatAsState(
                                    if (tools) 90f else -90f,
                                    tween(140),
                                    label = "Section direction",
                                )
                            SketchIcon(
                                "Back",
                                Modifier.size(16.dp).graphicsLayer { rotationZ = arrow },
                            )
                        }
                        MusicMenu(tools, { tools = false }) {
                            listOf("Home", "Search", "Library", "Devices", "Settings").forEach {
                                name ->
                                DropdownMenuItem(
                                    { Text(name) },
                                    {
                                        tools = false
                                        destination = name
                                        player = false
                                    },
                                )
                            }
                        }
                    }
                }
                Box(
                    Modifier.weight(1f)
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(22.dp))
                        .background(colors.surface)
                ) {
                    when {
                        player ->
                            PlayerPage(model, playing, queue, { queue = it }, { player = false })
                        destination == "Devices" -> DevicesPage(model, link, { link = it })
                        destination == "Settings" ->
                            SettingsPage(
                                model,
                                theme,
                                {
                                    theme = it
                                    preferences.edit().putString("theme", it).apply()
                                },
                                { destination = "Devices" },
                                colorSource,
                                { colorSource = it; preferences.edit().putString("colors", it).apply() },
                                { welcome=true },
                            )
                        detail != null -> {
                            val songs =
                                when {
                                    detail.startsWith("album:") ->
                                        library.songs.filter {
                                            it.albumId == detail.removePrefix("album:")
                                        }
                                    detail.startsWith("artist:") ->
                                        library.songs.filter {
                                            it.artist == detail.removePrefix("artist:")
                                        }
                                    detail.startsWith("collection:") ->
                                        library.collections
                                            .find { it.id == detail.removePrefix("collection:") }
                                            ?.songIds
                                            .orEmpty()
                                            .mapNotNull(model.repository::song)
                                    detail == "favorites" -> library.songs.filter { it.favorite }
                                    detail.startsWith("genre:") ->
                                        library.songs.filter {
                                            it.genre == detail.removePrefix("genre:")
                                        }
                                    else -> emptyList()
                                }
                            val title =
                                when {
                                    detail.startsWith("album:") -> songs.firstOrNull()?.album
                                    detail.startsWith("collection:") ->
                                        library.collections
                                            .find { it.id == detail.removePrefix("collection:") }
                                            ?.name
                                    detail == "favorites" -> "Favorites"
                                    else -> detail.substringAfter(':')
                                }.orEmpty()
                            LazyColumn(
                                Modifier.fillMaxSize().padding(horizontal = 12.dp),
                                state = detailScroll,
                                contentPadding = PaddingValues(bottom = 16.dp),
                            ) {
                                item {
                                    FilledTonalButton({ setDetail(null) }, shape = RoundedCornerShape(12.dp)) { SketchIcon("Back", Modifier.size(18.dp)); Text("Back") }
                                    if (detail.startsWith("album:"))
                                        Artwork(
                                            songs.firstOrNull()?.artworkFile,
                                            Modifier.width(180.dp).padding(bottom = 12.dp),
                                        )
                                    Text(
                                        title,
                                        style = MaterialTheme.typography.headlineMedium,
                                        fontWeight = FontWeight.Bold,
                                    )
                                    Text(
                                        "${songs.size} ${if(songs.size==1)"song" else "songs"}",
                                        color = colors.secondary,
                                    )
                                    MusicButton("Play", "Play") { model.play(songs) }
                                }
                                items(songs.size, key = { "$it/${songs[it].id}" }) { index ->
                                    SongRow(
                                        songs[index],
                                        { model.play(songs, index) },
                                        model,
                                        index,
                                    )
                                }
                            }
                        }
                        destination == "Search" -> {
                            val matches =
                                remember(library, query) {
                                    if (query.isBlank()) emptyList()
                                    else
                                        library.songs.filter {
                                            "${it.title} ${it.artist} ${it.album}"
                                                .contains(query, ignoreCase = true)
                                        }
                                }
                            Column(Modifier.padding(12.dp)) {
                                OutlinedTextField(
                                    query,
                                    { query = it },
                                    Modifier.fillMaxWidth(),
                                    singleLine = true,
                                    label = { Text("Search offline music") },
                                    leadingIcon = { SketchIcon("Search", Modifier.size(20.dp)) },
                                    shape = RoundedCornerShape(16.dp),
                                )
                                LazyColumn(state = searchScroll) {
                                    if (query.isNotBlank() && matches.isEmpty())
                                        item { Text("No matching songs", Modifier.padding(16.dp)) }
                                    items(matches, key = { it.id }) { song ->
                                        SongRow(
                                            song,
                                            { model.play(matches, matches.indexOf(song)) },
                                            model,
                                        )
                                    }
                                }
                            }
                        }
                        else ->
                            LazyColumn(
                                Modifier.fillMaxSize().padding(horizontal = 12.dp),
                                state = if (destination == "Home") homeScroll else libraryScroll,
                                contentPadding = PaddingValues(top = 16.dp, bottom = 16.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                item {
                                    Text(
                                        if (destination == "Home")
                                            library.displayName.ifBlank { "Home" }
                                        else "Library",
                                        style = MaterialTheme.typography.headlineLarge,
                                        fontWeight = FontWeight.Bold,
                                    )
                                    if(destination == "Home" && library.songs.isNotEmpty()) Text("${songCount(library.songs.count { it.available })} ready offline", fontSize = 13.sp, color = colors.secondary)
                                }
                                if (library.songs.isEmpty()) {
                                    item {
                                        Column(Modifier.padding(vertical = 24.dp)) {
                                            Text(
                                                "Your next obsession, to go",
                                                style = MaterialTheme.typography.titleLarge,
                                            )
                                            Text(
                                                "Pair your Mac over Wi-Fi and copy songs, artwork and collections to this phone.",
                                                Modifier.padding(vertical = 12.dp),
                                                color = colors.secondary,
                                            )
                                            MusicButton("Pair a Mac", "device") {
                                                destination = "Devices"
                                            }
                                        }
                                    }
                                } else if (destination == "Home") {
                                    if (playing.song != null)
                                        item {
                                            Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(18.dp))
                                                .background(colors.control).padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
                                                Artwork(playing.song!!.artworkFile, Modifier.size(64.dp).clickable { player = true; queue = false })
                                                Column(Modifier.weight(1f).padding(horizontal = 12.dp).clickable { player = true; queue = false }) {
                                                    Text(if(playing.playing) "Now playing" else "Continue listening", fontSize = 12.sp, color = colors.secondary)
                                                    Text(playing.song!!.title, maxLines = 2, overflow = TextOverflow.Ellipsis, fontWeight = FontWeight.Bold)
                                                    Text(playing.song!!.artist, maxLines = 1, overflow = TextOverflow.Ellipsis, fontSize = 13.sp, color = colors.secondary)
                                                }
                                                IconButton(model::toggle, Modifier.size(48.dp).semantics { contentDescription = if(playing.playing) "Pause saved song" else "Resume saved song" }) {
                                                    PlayPauseIcon(playing.playing, Modifier.size(34.dp))
                                                }
                                            }
                                        }
                                    item {
                                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                            Text("Collections", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                                            DeviceButton("All") { destination = "Library"; category = "Collections" }
                                        }
                                        LazyRow(horizontalArrangement = Arrangement.spacedBy(12.dp), contentPadding = PaddingValues(vertical = 8.dp)) {
                                            item {
                                                Column(Modifier.width(112.dp).clickable { setDetail("favorites") }) {
                                                    Box(Modifier.size(112.dp).clip(RoundedCornerShape(16.dp)).background(colors.control), contentAlignment = Alignment.Center) {
                                                        SketchIcon("Heart-Filled", Modifier.size(32.dp), colors.accent)
                                                    }
                                                    Text("Favorites", Modifier.padding(top = 6.dp), fontWeight = FontWeight.SemiBold, maxLines = 1)
                                                    Text(songCount(library.songs.count { it.favorite }), fontSize = 12.sp, color = colors.secondary)
                                                }
                                            }
                                            items(library.collections, key = { it.id }) { c ->
                                                Column(Modifier.width(112.dp).clickable { setDetail("collection:${c.id}") }) {
                                                    Artwork(c.songIds.firstNotNullOfOrNull { id -> library.songs.find { it.id == id }?.artworkFile }, Modifier.size(112.dp))
                                                    Text(c.name, Modifier.padding(top = 6.dp), fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
                                                    Text(songCount(c.songIds.size), fontSize = 12.sp, color = colors.secondary)
                                                }
                                            }
                                        }
                                    }
                                    item {
                                        Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                                            Text("Albums", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                                            DeviceButton("All albums") { destination = "Library"; category = "Albums" }
                                        }
                                        LazyRow(
                                            horizontalArrangement = Arrangement.spacedBy(12.dp)
                                        ) {
                                            items(
                                                library.songs
                                                    .groupBy { it.albumId }
                                                    .values
                                                    .toList(),
                                                key = { it.first().albumId },
                                            ) { songs ->
                                                Column(
                                                    Modifier.width(140.dp).clickable {
                                                        setDetail("album:${songs.first().albumId}")
                                                    }
                                                ) {
                                                    Artwork(songs.first().artworkFile)
                                                    Text(
                                                        songs.first().album,
                                                        maxLines = 2,
                                                        overflow = TextOverflow.Ellipsis,
                                                        fontWeight = FontWeight.SemiBold,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                    item {
                                        Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                                            Text("Songs", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                                            DeviceButton("All songs") { destination = "Library"; category = "Songs" }
                                        }
                                    }
                                    items(library.songs.take(6), key = { it.id }) { song ->
                                        SongRow(song, { model.play(listOf(song)) }, model)
                                    }
                                } else {
                                    item {
                                        Row(Modifier.fillMaxWidth().padding(bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                                            listOf("Albums", "Artists", "Songs").forEach { name ->
                                                TextButton({ category = name }, Modifier.weight(1f), shape = RoundedCornerShape(12.dp),
                                                    colors = ButtonDefaults.textButtonColors(containerColor = if(category == name) colors.control else androidx.compose.ui.graphics.Color.Transparent)) {
                                                    Text(name, color = colors.ink, fontWeight = if(category == name) FontWeight.Bold else FontWeight.Normal)
                                                }
                                            }
                                            var more by remember { mutableStateOf(false) }
                                            Box {
                                                IconButton({ more = true }, Modifier.size(48.dp).semantics { contentDescription = "More library views" }) { SketchIcon("More", Modifier.size(22.dp)) }
                                                MusicMenu(more, { more = false }) {
                                                    listOf("Collections", "Favorites", "Genres").forEach { name ->
                                                        DropdownMenuItem({ Text(name) }, { category = name; more = false })
                                                    }
                                                }
                                            }
                                        }
                                        if(category !in listOf("Albums", "Artists", "Songs")) Text(category, Modifier.padding(bottom = 8.dp), fontWeight = FontWeight.SemiBold)

                                    }
                                    when (category) {
                                        "Songs",
                                        "Favorites" -> {
                                            val songs =
                                                library.songs.filter {
                                                    category != "Favorites" || it.favorite
                                                }
                                            items(songs, key = { it.id }) { song ->
                                                SongRow(
                                                    song,
                                                    { model.play(songs, songs.indexOf(song)) },
                                                    model,
                                                )
                                            }
                                        }
                                        "Collections" ->
                                            items(library.collections, key = { it.id }) { c ->
                                                MusicButton(
                                                    "${c.name} · ${c.songIds.size} songs",
                                                    "Library",
                                                ) {
                                                    setDetail("collection:${c.id}")
                                                }
                                            }
                                        else -> {
                                            val groups =
                                                library.songs
                                                    .groupBy {
                                                        when (category) {
                                                            "Artists" -> it.artist
                                                            "Genres" -> it.genre.ifBlank { "Other" }
                                                            else -> it.albumId
                                                        }
                                                    }
                                                    .entries
                                                    .toList()
                                            if (category == "Albums") {
                                                items(groups.chunked(2), key = { it.first().key }) { pair ->
                                                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                                        pair.forEach { (_, songs) ->
                                                            Column(Modifier.weight(1f).padding(bottom = 8.dp).clickable { setDetail("album:${songs.first().albumId}") }) {
                                                                Artwork(songs.first().artworkFile, Modifier.fillMaxWidth())
                                                                Text(songs.first().album, Modifier.padding(top = 6.dp), maxLines = 2, overflow = TextOverflow.Ellipsis, fontWeight = FontWeight.SemiBold)
                                                                Text(songs.first().artist, fontSize = 12.sp, color = colors.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                                            }
                                                        }
                                                        if(pair.size == 1) Spacer(Modifier.weight(1f))
                                                    }
                                                }
                                            } else
                                            items(groups, key = { it.key }) { (key, songs) ->
                                                Row(
                                                    Modifier.fillMaxWidth()
                                                        .clip(RoundedCornerShape(16.dp))
                                                        .background(
                                                            colors.control.copy(alpha = .4f)
                                                        )
                                                        .clickable {
                                                            setDetail(
                                                                when (category) {
                                                                    "Artists" -> "artist:$key"
                                                                    "Genres" -> "genre:$key"
                                                                    else -> "album:$key"
                                                                }
                                                            )
                                                        }
                                                        .padding(10.dp),
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    Artwork(
                                                        songs.first().artworkFile,
                                                        Modifier.size(64.dp),
                                                    )
                                                    Column(
                                                        Modifier.weight(1f).padding(start = 12.dp)
                                                    ) {
                                                        Text(
                                                            if (category == "Albums")
                                                                songs.first().album
                                                            else key,
                                                            fontWeight = FontWeight.SemiBold,
                                                        )
                                                        Text(
                                                            "${songs.size} ${if(songs.size==1)"song" else "songs"}",
                                                            color = colors.secondary,
                                                            fontSize = 12.sp,
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                    }
                }
                if (!player) {
                    Row(
                        Modifier.fillMaxWidth()
                            .padding(top = 6.dp)
                            .clip(RoundedCornerShape(18.dp))
                            .background(colors.surface)
                            .padding(4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Artwork(
                            playing.song?.artworkFile,
                            Modifier.size(42.dp).clickable {
                                player = true
                                queue = false
                            },
                        )
                        Column(
                            Modifier.weight(1f)
                                .clickable {
                                    player = true
                                    queue = false
                                }
                                .padding(horizontal = 8.dp)
                        ) {
                            Text(
                                playing.song?.title ?: "Nothing playing",
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 14.sp,
                            )
                            Text(
                                playing.song?.artist ?: "Choose offline music",
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                fontSize = 11.sp,
                                color = colors.secondary,
                            )
                        }
                        IconButton(
                            { playing.song?.let { model.repository.favorite(it.id) } },
                            Modifier.size(44.dp).semantics {
                                contentDescription =
                                    if (playing.song?.favorite == true) "Remove favorite"
                                    else "Favorite"
                            },
                        ) {
                            BeatingHeart(playing.song?.favorite == true, playing.playing, Modifier.size(16.dp))
                        }
                        IconButton(
                            model::toggle,
                            Modifier.semantics {
                                contentDescription = if (playing.playing) "Pause" else "Play"
                            },
                        ) {
                            PlayPauseIcon(playing.playing, Modifier.size(28.dp))
                        }
                        IconAction("Next", "Next", model::next)
                        IconButton(
                            {
                                player = true
                                queue = true
                            },
                            Modifier.semantics { contentDescription = "Up next" },
                        ) {
                            SketchIcon("Queue", Modifier.size(22.dp))
                        }
                    }
                }
                Row(
                    Modifier.fillMaxWidth().height(64.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    listOf("Home" to "Home", "Search" to "Search", "Library" to "Library")
                        .forEach { (name, icon) ->
                            val reaction = remember(name) { Animatable(0f) }
                            val reactions = rememberCoroutineScope()
                            val interaction = remember(name) { MutableInteractionSource() }
                            Column(
                                Modifier.weight(1f)
                                    .clip(RoundedCornerShape(16.dp))
                                    .background(
                                        if (destination == name) colors.control else colors.shell
                                    )
                                    .testTag("nav-$name")
                                    .clickable(interactionSource = interaction, indication = null) {
                                        destination = name
                                        player = false
                                        if (android.animation.ValueAnimator.areAnimatorsEnabled())
                                            reactions.launch {
                                                reaction.animateTo(
                                                    0f,
                                                    keyframes {
                                                        durationMillis = 180
                                                        -9f at 58
                                                        4f at 122
                                                        0f at 180
                                                    },
                                                )
                                            }
                                    }
                                    .padding(vertical = 6.dp),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                SketchIcon(
                                    icon,
                                    Modifier.size(24.dp).graphicsLayer {
                                        rotationZ = reaction.value
                                    },
                                )
                                Text(
                                    name,
                                    fontSize = 11.sp,
                                    fontWeight =
                                        if (destination == name) FontWeight.Bold
                                        else FontWeight.Normal,
                                )
                            }
                        }
                }
            }
        }
        if (message != null)
            AlertDialog(
                { model.message.value = null },
                confirmButton = { TextButton({ model.message.value = null }) { Text("OK") } },
                title = { Text("musicOS") },
                text = { Text(message!!) },
            )
    }
}

@Composable
internal fun PlayerPage(
    model: PhoneModel,
    state: Playing,
    queue: Boolean,
    onQueue: (Boolean) -> Unit,
    onClose: () -> Unit,
) {
    val colors = LocalMusicColors.current
    var volumeOpen by rememberSaveable { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Row(Modifier.clip(RoundedCornerShape(14.dp)).background(colors.control).padding(2.dp)) {
                listOf(false to "Now playing", true to "Up next").forEach { (value, label) ->
                    TextButton({ onQueue(value) }, Modifier.heightIn(min = 48.dp), shape = RoundedCornerShape(12.dp),
                        colors = ButtonDefaults.textButtonColors(containerColor = if (queue == value) colors.surface else androidx.compose.ui.graphics.Color.Transparent)) {
                        Text(label, color = if (queue == value) colors.ink else colors.secondary,
                            fontWeight = if (queue == value) FontWeight.SemiBold else FontWeight.Normal)
                    }
                }
            }
            Spacer(Modifier.weight(1f))
            IconButton(onClose, Modifier.size(48.dp).semantics { contentDescription = "Close player" }) {
                SketchIcon("Back", Modifier.size(20.dp).graphicsLayer { rotationZ = -90f }, colors.secondary)
            }
        }
        if (queue) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Up next", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge)
                TextButton(model::clearUpcoming) { Text("Clear next") }
            }
            LazyColumn(Modifier.weight(1f)) {
                items(state.queue, key = { it.entry }) { entry ->
                    Row(
                        Modifier.fillMaxWidth()
                            .padding(vertical = 3.dp)
                            .clip(RoundedCornerShape(14.dp))
                            .background(
                                if (entry.index == state.current) colors.control else colors.surface
                            ),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(
                            Modifier.weight(1f)
                                .clickable { model.select(entry.index) }
                                .padding(10.dp)
                        ) {
                            Text(
                                entry.song?.title ?: "Unavailable song",
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                fontWeight = FontWeight.SemiBold,
                            )
                            Text(
                                entry.song?.artist.orEmpty(),
                                fontSize = 11.sp,
                                color = colors.secondary,
                            )
                        }
                        IconButton(
                            { if (entry.index > 0) model.move(entry.index, entry.index - 1) },
                            enabled = entry.index > 0,
                        ) {
                            Text("↑")
                        }
                        IconButton(
                            {
                                if (entry.index < state.queue.lastIndex)
                                    model.move(entry.index, entry.index + 1)
                            },
                            enabled = entry.index < state.queue.lastIndex,
                        ) {
                            Text("↓")
                        }
                        IconButton({ model.remove(entry.index) }) { Text("×") }
                    }
                }
            }
        } else {
            BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                val short = maxHeight < 500.dp
                Column(Modifier.widthIn(max = 420.dp).fillMaxWidth()
                    .then(if(short) Modifier.verticalScroll(rememberScrollState()) else Modifier.fillMaxHeight())
                    .padding(horizontal = 8.dp, vertical = 6.dp),
                    verticalArrangement = Arrangement.spacedBy(if (short) 6.dp else 12.dp, Alignment.CenterVertically)) {
                    if (short) Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Artwork(state.song?.artworkFile, Modifier.size(80.dp).testTag("player-artwork"))
                        PlayerIdentity(model, state, Modifier.weight(1f), compact = true)
                    } else {
                        Box(Modifier.weight(1f, fill = false).fillMaxWidth(), contentAlignment = Alignment.Center) {
                            Artwork(state.song?.artworkFile, Modifier.heightIn(max = 300.dp).fillMaxHeight().aspectRatio(1f, matchHeightConstraintsFirst = true).testTag("player-artwork"))
                        }
                        PlayerIdentity(model, state, Modifier.fillMaxWidth())
                    }
                    PlayerSeek(model, state)
                    Transport(model, state, true)
                    Row(Modifier.widthIn(max = 300.dp).fillMaxWidth().align(Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
                        FilledTonalButton({ volumeOpen = !volumeOpen }, Modifier.heightIn(min = 48.dp)
                            .semantics { contentDescription = "Volume controls" }, shape = RoundedCornerShape(14.dp)) {
                            Text(if(volumeOpen) "Volume" else "Volume · ${(state.volume * 100).toInt()}%", color = colors.secondary, fontSize = 12.sp)
                        }
                        if (volumeOpen) PlayerSlider(state.volume, model::volume, Modifier.weight(1f), "Listening volume")
                    }
                    if (state.error != null) Text(state.error, color = MaterialTheme.colorScheme.error)
                }
            }
        }
        if (queue) {
            PlayerSeek(model, state)
            Transport(model, state)
        }
    }
}

@Composable
private fun PlayerIdentity(model: PhoneModel, state: Playing, modifier: Modifier, compact: Boolean = false) {
    val colors = LocalMusicColors.current
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f).testTag("player-identity")) {
            Text(state.song?.title ?: "Nothing playing", fontSize = if(compact) 20.sp else 22.sp, color = colors.ink,
                lineHeight = if(compact) 24.sp else 26.sp, fontWeight = FontWeight.Bold, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Text(state.song?.let { "${it.artist} · ${it.album}" } ?: "Choose music in Library",
                fontSize = 14.sp, color = colors.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
        IconButton({ state.song?.let { model.repository.favorite(it.id) } },
            Modifier.size(48.dp).semantics { contentDescription = "Favorite playing song" }, enabled = state.song != null) {
            BeatingHeart(state.song?.favorite == true, state.playing, Modifier.size(20.dp))
        }
    }
}

@Composable
private fun PlayerSeek(model: PhoneModel, state: Playing) {
    var seeking by remember(state.song?.id) { mutableStateOf<Float?>(null) }
    Column(Modifier.fillMaxWidth()) {
        PlayerSlider(seeking ?: state.position.toFloat(), { seeking = it }, Modifier.fillMaxWidth(),
            "Playback position", 0f..state.duration.toFloat().coerceAtLeast(1f), {
                seeking?.let { model.seek(it.toLong()) }
                seeking = null
            })
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(time(state.position), fontSize = 12.sp, color = LocalMusicColors.current.secondary)
            Text(time(state.duration), fontSize = 12.sp, color = LocalMusicColors.current.secondary)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PlayerSlider(value: Float, change: (Float) -> Unit, modifier: Modifier, label: String,
    range: ClosedFloatingPointRange<Float> = 0f..1f, finished: (() -> Unit)? = null) {
    Slider(value, change, modifier.heightIn(min = 48.dp).semantics { contentDescription = label },
        valueRange = range, onValueChangeFinished = finished,
        thumb = { Box(Modifier.size(12.dp).clip(RoundedCornerShape(6.dp)).background(LocalMusicColors.current.accent)) },
        track = { state -> SliderDefaults.Track(state, Modifier.height(3.dp), thumbTrackGapSize = 0.dp, drawStopIndicator = null) })
}

@Composable
private fun SettingsPage(
    model: PhoneModel,
    theme: String,
    onTheme: (String) -> Unit,
    onDevices: () -> Unit,
    colorSource: String,
    onColorSource: (String) -> Unit,
    onSetup: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    LazyColumn(
        Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Text(
                "Settings",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
            )
            Text("Appearance", style = MaterialTheme.typography.titleLarge)
            Row {
                listOf("system", "light", "dark").forEach { value ->
                    FilterChip(
                        theme == value,
                        { onTheme(value) },
                        label = { Text(value.replaceFirstChar { it.uppercase() }) },
                        modifier = Modifier.padding(end = 6.dp),
                    )
                }
            }
        }
        item {
            Text("Colors", style = MaterialTheme.typography.titleLarge)
            Text("Keep the musicOS blue or follow your current artwork.", fontSize = 13.sp, color = LocalMusicColors.current.secondary)
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                listOf("blue" to "musicOS blue", "artwork" to "From artwork").forEach { (value,label) ->
                    FilterChip(colorSource == value, { onColorSource(value) }, label = { Text(label) }, shape = RoundedCornerShape(12.dp))
                }
            }
        }
        item {
            Text("Audio", style = MaterialTheme.typography.titleLarge)
            Text(
                "Lock-screen, notification, headset and Bluetooth controls use Android’s media session. Playback pauses when headphones disconnect.",
                fontSize = 13.sp,
            )
            MusicButton("Audio output", "device") {
                runCatching {
                        if (Build.VERSION.SDK_INT >= 34)
                            android.media.MediaRouter2.getInstance(context)
                                .showSystemOutputSwitcher()
                        else context.startActivity(Intent(Settings.ACTION_SOUND_SETTINGS))
                    }
                    .onFailure {
                        model.message.value = "Open Android sound settings to choose an output."
                    }
            }
            MusicButton("System equalizer") {
                val controller = model.controller
                if (controller == null) model.message.value = "Choose a song first."
                else {
                    val future =
                        controller.sendCustomCommand(
                            SessionCommand("musicos.audio_session", Bundle.EMPTY),
                            Bundle.EMPTY,
                        )
                    future.addListener(
                        {
                            runCatching {
                                    val id = future.get().extras.getInt("session", 0)
                                    require(id > 0)
                                    context.startActivity(
                                        Intent(
                                                AudioEffect
                                                    .ACTION_DISPLAY_AUDIO_EFFECT_CONTROL_PANEL
                                            )
                                            .putExtra(AudioEffect.EXTRA_AUDIO_SESSION, id)
                                            .putExtra(
                                                AudioEffect.EXTRA_PACKAGE_NAME,
                                                context.packageName,
                                            )
                                            .putExtra(
                                                AudioEffect.EXTRA_CONTENT_TYPE,
                                                AudioEffect.CONTENT_TYPE_MUSIC,
                                            )
                                    )
                                }
                                .onFailure {
                                    model.message.value =
                                        "This device has no system equalizer available for musicOS."
                                }
                        },
                        androidx.core.content.ContextCompat.getMainExecutor(context),
                    )
                }
            }
        }
        item {
            Text("Offline music", style = MaterialTheme.typography.titleLarge)
            MusicButton("Manage paired Macs", "device", onDevices)
            Text(
                "Only verified files enter your library. Interrupted transfers resume; removing a pairing keeps downloaded music.",
                fontSize = 13.sp,
            )
        }
        item {
            MusicButton("Android notification settings") {
                context.startActivity(
                    Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                )
            }
            MusicButton("Open welcome setup", "Album", onSetup)
            Text("${PhoneProduct.load(context).name} · ${PhoneProduct.load(context).version} · Figtree · supplied Sketch icons", fontSize = 12.sp)
        }
    }
}
