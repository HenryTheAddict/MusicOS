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
        incoming.value = value?.data?.toString().orEmpty()
        voice.value =
            if (value?.action == android.provider.MediaStore.INTENT_ACTION_MEDIA_PLAY_FROM_SEARCH)
                value.getStringExtra(android.app.SearchManager.QUERY).orEmpty()
            else ""
    }
}

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
        IconButton({ state.song?.let { model.repository.favorite(it.id) } }, Modifier.size(48.dp)) {
            SketchIcon(
                if (state.song?.favorite == true) "Heart-Filled" else "Heart",
                Modifier.size(20.dp),
                LocalMusicColors.current.accent,
            )
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
        IconButton(model::shuffle, Modifier.size(48.dp)) {
            SketchIcon(
                "Shuffle",
                color =
                    if (state.shuffle) LocalMusicColors.current.accent
                    else LocalMusicColors.current.secondary,
            )
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
            DropdownMenu(menu, { menu = false }) {
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
    var theme by rememberSaveable {
        mutableStateOf(preferences.getString("theme", "system") ?: "system")
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
    MusicTheme(dark) {
        val colors = LocalMusicColors.current
        val library by model.library.collectAsStateWithLifecycle()
        val playing by model.playing.collectAsStateWithLifecycle()
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
        Surface(Modifier.fillMaxSize(), color = colors.shell) {
            Column(Modifier.fillMaxSize().safeDrawingPadding().padding(horizontal = 8.dp)) {
                Row(
                    Modifier.fillMaxWidth().height(56.dp).padding(horizontal = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    SketchIcon("Album", Modifier.size(26.dp))
                    Text(
                        "MusicOS",
                        Modifier.weight(1f).padding(start = 8.dp),
                        fontWeight = FontWeight.Bold,
                        fontSize = 20.sp,
                    )
                    Box {
                        TextButton({ tools = true }) {
                            Text(destination)
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
                        DropdownMenu(tools, { tools = false }) {
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
                                    TextButton({ setDetail(null) }) { Text("‹ Back") }
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
                                            Column(
                                                Modifier.clip(RoundedCornerShape(18.dp))
                                                    .background(colors.control)
                                                    .padding(14.dp)
                                            ) {
                                                Text("Continue listening", fontSize = 12.sp)
                                                Text(
                                                    playing.song!!.title,
                                                    maxLines = 2,
                                                    fontWeight = FontWeight.Bold,
                                                )
                                                Text(
                                                    playing.song!!.artist,
                                                    color = colors.secondary,
                                                )
                                                MusicButton(
                                                    if (playing.playing) "Pause" else "Resume",
                                                    if (playing.playing) "Pause" else "Play",
                                                    model::toggle,
                                                )
                                            }
                                        }
                                    item {
                                        Text(
                                            "Collections",
                                            style = MaterialTheme.typography.titleLarge,
                                        )
                                        LazyRow(
                                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                                        ) {
                                            item {
                                                MusicButton("Favorites", "Heart") {
                                                    setDetail("favorites")
                                                }
                                            }
                                            items(library.collections, key = { it.id }) { c ->
                                                MusicButton(c.name) {
                                                    setDetail("collection:${c.id}")
                                                }
                                            }
                                        }
                                    }
                                    item {
                                        Text("Albums", style = MaterialTheme.typography.titleLarge)
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
                                        Text("Songs", style = MaterialTheme.typography.titleLarge)
                                    }
                                    items(library.songs.take(30), key = { it.id }) { song ->
                                        SongRow(song, { model.play(listOf(song)) }, model)
                                    }
                                } else {
                                    item {
                                        LazyRow(
                                            horizontalArrangement = Arrangement.spacedBy(6.dp)
                                        ) {
                                            items(
                                                listOf(
                                                    "Albums",
                                                    "Artists",
                                                    "Songs",
                                                    "Collections",
                                                    "Favorites",
                                                    "Genres",
                                                )
                                            ) { name ->
                                                FilterChip(
                                                    category == name,
                                                    { category = name },
                                                    label = { Text(name) },
                                                )
                                            }
                                        }
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
                            SketchIcon(
                                if (playing.song?.favorite == true) "Heart-Filled" else "Heart",
                                Modifier.size(16.dp),
                                colors.accent,
                            )
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
                            Column(
                                Modifier.weight(1f)
                                    .clip(RoundedCornerShape(16.dp))
                                    .background(
                                        if (destination == name) colors.control else colors.shell
                                    )
                                    .testTag("nav-$name")
                                    .clickable {
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
                title = { Text("MusicOS") },
                text = { Text(message!!) },
            )
    }
}

@Composable
private fun PlayerPage(
    model: PhoneModel,
    state: Playing,
    queue: Boolean,
    onQueue: (Boolean) -> Unit,
    onClose: () -> Unit,
) {
    val colors = LocalMusicColors.current
    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClose) { Text("⌄ Close") }
            Spacer(Modifier.weight(1f))
            FilterChip(!queue, { onQueue(false) }, label = { Text("Now playing") })
            Spacer(Modifier.width(6.dp))
            FilterChip(queue, { onQueue(true) }, label = { Text("Up next") })
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
            LazyColumn(
                Modifier.weight(1f),
                horizontalAlignment = Alignment.CenterHorizontally,
                contentPadding = PaddingValues(bottom = 12.dp),
            ) {
                item {
                    Artwork(
                        state.song?.artworkFile,
                        Modifier.widthIn(max = 300.dp).fillMaxWidth(.8f).padding(vertical = 12.dp),
                    )
                    Text(
                        state.song?.title ?: "Nothing playing",
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(state.song?.artist.orEmpty(), color = colors.secondary)
                    Text(state.song?.album.orEmpty(), fontSize = 12.sp, color = colors.secondary)
                    var seeking by remember { mutableStateOf<Float?>(null) }
                    Slider(
                        seeking ?: state.position.toFloat(),
                        { seeking = it },
                        onValueChangeFinished = {
                            seeking?.let { model.seek(it.toLong()) }
                            seeking = null
                        },
                        valueRange = 0f..state.duration.toFloat().coerceAtLeast(1f),
                    )
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(time(state.position), fontSize = 12.sp)
                        Text(time(state.duration), fontSize = 12.sp)
                    }
                    if (state.error != null)
                        Text(state.error, color = MaterialTheme.colorScheme.error)
                    Transport(model, state, true)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Volume", fontSize = 12.sp)
                        Slider(state.volume, model::volume, Modifier.weight(1f))
                        Text("${(state.volume*100).toInt()}%", fontSize = 12.sp)
                    }
                    MusicButton(
                        when (state.repeat) {
                            Player.REPEAT_MODE_ONE -> "Repeat one"
                            Player.REPEAT_MODE_ALL -> "Repeat all"
                            else -> "Repeat off"
                        },
                        "Repeat",
                        model::repeat,
                    )
                }
            }
        }
        if (queue) Transport(model, state)
    }
}

@Composable
private fun DevicesPage(model: PhoneModel, link: String, onLink: (String) -> Unit) {
    val macs by model.macs.collectAsStateWithLifecycle()
    val pairing by model.pairing.collectAsStateWithLifecycle()
    val status by model.pairingStatus.collectAsStateWithLifecycle()
    val jobs by model.jobs.collectAsStateWithLifecycle()
    val context = LocalContext.current
    var pending by remember { mutableStateOf<(() -> Unit)?>(null) }
    val permission =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { _
            ->
            val action = pending
            pending = null
            if (
                Build.VERSION.SDK_INT < 37 ||
                    androidx.core.content.ContextCompat.checkSelfPermission(
                        context,
                        "android.permission.ACCESS_LOCAL_NETWORK",
                    ) == android.content.pm.PackageManager.PERMISSION_GRANTED
            )
                action?.invoke()
            else
                model.message.value =
                    "Local network access is needed to connect to the Mac. Enable it in Android Settings to retry."
        }
    fun network(action: () -> Unit) {
        val permissions =
            buildList {
                    if (Build.VERSION.SDK_INT >= 37) add("android.permission.ACCESS_LOCAL_NETWORK")
                    if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
                }
                .filter {
                    androidx.core.content.ContextCompat.checkSelfPermission(context, it) !=
                        android.content.pm.PackageManager.PERMISSION_GRANTED
                }
        if (permissions.isEmpty()) action()
        else {
            pending = action
            permission.launch(permissions.toTypedArray())
        }
    }
    LazyColumn(
        Modifier.fillMaxSize().padding(horizontal = 14.dp),
        contentPadding = PaddingValues(vertical = 18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Text(
                "Devices",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "Copy from your Mac over Wi-Fi. Your songs keep playing when the Mac is offline.",
                color = LocalMusicColors.current.secondary,
            )
        }
        item {
            Column(
                Modifier.fillMaxWidth()
                    .clip(RoundedCornerShape(18.dp))
                    .background(LocalMusicColors.current.control)
                    .padding(14.dp)
            ) {
                Text("Pair a Mac", style = MaterialTheme.typography.titleLarge)
                Text(
                    "On the Mac, open Devices, choose music, then create a pairing link. Paste or open that link here.",
                    fontSize = 13.sp,
                )
                OutlinedTextField(
                    link,
                    onLink,
                    Modifier.fillMaxWidth().padding(vertical = 8.dp),
                    label = { Text("Pairing link") },
                    maxLines = 4,
                    enabled = !pairing,
                )
                Button({ network { model.pair(link) } }, enabled = !pairing && link.isNotBlank()) {
                    Text(if (pairing) "Waiting for approval…" else "Connect to Mac")
                }
                if (status.isNotBlank())
                    Text(status, Modifier.padding(top = 8.dp), fontSize = 13.sp)
            }
        }
        items(macs, key = { it.id }) { mac ->
            Column(
                Modifier.fillMaxWidth()
                    .clip(RoundedCornerShape(18.dp))
                    .background(LocalMusicColors.current.control.copy(alpha = .5f))
                    .padding(14.dp)
            ) {
                Text(mac.name, fontWeight = FontWeight.Bold, fontSize = 20.sp)
                Text("Paired · offline copies stay on this phone", fontSize = 12.sp)
                Row {
                    MusicButton("Copy music", "sync") { network { model.sync(mac) } }
                    TextButton({ model.forget(mac) }) { Text("Forget") }
                }
                val id = jobs[mac.id]
                if (id != null) {
                    val info by
                        model.work
                            .getWorkInfoByIdFlow(id)
                            .collectAsStateWithLifecycle(initialValue = null)
                    val job = info
                    if (job != null) {
                        val done = job.progress.getLong("done", 0)
                        val total = job.progress.getLong("total", 0)
                        Text(
                            when (job.state) {
                                WorkInfo.State.SUCCEEDED -> "Offline music is ready"
                                WorkInfo.State.FAILED ->
                                    job.outputData.getString("error")
                                        ?: "Copy failed. Retry while the Mac is sharing."
                                WorkInfo.State.CANCELLED -> "Stopped. Finished copies are kept."
                                WorkInfo.State.ENQUEUED ->
                                    "Waiting for unmetered Wi-Fi or retrying connection…"
                                else -> job.progress.getString("message") ?: "Preparing copies…"
                            },
                            fontSize = 13.sp,
                        )
                        if (!job.state.isFinished) {
                            if (total > 0)
                                LinearProgressIndicator(
                                    progress = { done.toFloat() / total },
                                    modifier = Modifier.fillMaxWidth(),
                                )
                            else LinearProgressIndicator(Modifier.fillMaxWidth())
                            TextButton({ model.cancel(mac) }) { Text("Stop copying") }
                        }
                    }
                }
            }
        }
        item {
            Text(
                "Music is copied without changing the files on your Mac. New syncs keep existing offline copies and preserve favorites changed on this phone. No cloud account is required.",
                fontSize = 12.sp,
                color = LocalMusicColors.current.secondary,
            )
        }
    }
}

@Composable
private fun SettingsPage(
    model: PhoneModel,
    theme: String,
    onTheme: (String) -> Unit,
    onDevices: () -> Unit,
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
                                        "This device has no system equalizer available for MusicOS."
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
            Text("MusicOS · Figtree · supplied Sketch icons", fontSize = 12.sp)
        }
    }
}
