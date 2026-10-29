package com.musicos.android

import android.graphics.BitmapFactory
import android.graphics.ColorSpace
import android.os.Build
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.caverock.androidsvg.SVG
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

@Immutable
data class MusicColors(
    val shell: Color,
    val surface: Color,
    val control: Color,
    val ink: Color,
    val secondary: Color,
    val accent: Color,
    val accentInk: Color,
    val playbackTop: Color,
    val playbackBottom: Color,
)

val LocalMusicColors = staticCompositionLocalOf {
    MusicColors(
        Color(0xff00baff),
        Color(0xffcbf1ff),
        Color(0xffa6d7e8),
        Color(0xff12202a),
        Color(0xff425963),
        Color(0xff0064df),
        Color.White,
        Color(0xff305764),
        Color(0xff203b49),
    )
}

@Composable
fun MusicTheme(dark: Boolean, artwork: String? = null, content: @Composable () -> Unit) {
    val artworkColor by produceState<Int?>(null, artwork) { value = withContext(Dispatchers.IO) { artwork?.let(::artworkSeed) } }
    val baseColors =
        if (dark)
            MusicColors(
                Color(0xff222638),
                Color(0xff282c3f),
                Color(0xff383f59),
                Color(0xffe0e2ee),
                Color(0xffb4bbd5),
                Color(0xffa4aff2),
                Color(0xff202438),
                Color(0xffb5bde6),
                Color(0xff8d97ca),
            )
        else LocalMusicColors.current
    val colors = artworkColor?.let { companionColors(it, dark) } ?: baseColors
    val font = FontFamily(Font(R.font.figtree))
    val base = Typography()
    val typography =
        Typography(
            displayLarge = base.displayLarge.copy(fontFamily = font),
            displayMedium = base.displayMedium.copy(fontFamily = font),
            displaySmall = base.displaySmall.copy(fontFamily = font),
            headlineLarge = base.headlineLarge.copy(fontFamily = font),
            headlineMedium = base.headlineMedium.copy(fontFamily = font),
            headlineSmall = base.headlineSmall.copy(fontFamily = font),
            titleLarge = base.titleLarge.copy(fontFamily = font),
            titleMedium = base.titleMedium.copy(fontFamily = font),
            titleSmall = base.titleSmall.copy(fontFamily = font),
            bodyLarge = base.bodyLarge.copy(fontFamily = font),
            bodyMedium = base.bodyMedium.copy(fontFamily = font),
            bodySmall = base.bodySmall.copy(fontFamily = font),
            labelLarge = base.labelLarge.copy(fontFamily = font),
            labelMedium = base.labelMedium.copy(fontFamily = font),
            labelSmall = base.labelSmall.copy(fontFamily = font),
        )
    val scheme =
        if (dark)
            darkColorScheme(
                primary = colors.accent,
                onPrimary = colors.accentInk,
                surface = colors.surface,
                onSurface = colors.ink,
                onSurfaceVariant = colors.secondary,
                surfaceVariant = colors.control,
                secondaryContainer = colors.control,
                onSecondaryContainer = colors.ink,
                surfaceContainer = colors.surface,
                surfaceContainerHigh = colors.control,
                background = colors.shell,
                onBackground = colors.ink,
            )
        else
            lightColorScheme(
                primary = colors.accent,
                onPrimary = colors.accentInk,
                surface = colors.surface,
                onSurface = colors.ink,
                onSurfaceVariant = colors.secondary,
                surfaceVariant = colors.control,
                secondaryContainer = colors.control,
                onSecondaryContainer = colors.ink,
                surfaceContainer = colors.surface,
                surfaceContainerHigh = colors.control,
                background = colors.shell,
                onBackground = colors.ink,
            )
    CompositionLocalProvider(LocalMusicColors provides colors) {
        MaterialTheme(colorScheme = scheme, typography = typography, shapes = androidx.compose.material3.Shapes(extraSmall = RoundedCornerShape(12.dp), small = RoundedCornerShape(14.dp), medium = RoundedCornerShape(18.dp), large = RoundedCornerShape(22.dp), extraLarge = RoundedCornerShape(28.dp)), content = content)
    }
}

private fun Color.hex() = "#%06X".format(toArgb() and 0xffffff)

@Composable
fun SketchIcon(
    name: String,
    modifier: Modifier = Modifier.size(24.dp),
    color: Color = LocalMusicColors.current.ink,
) {
    val assetName =
        when (name) {
            "Search" -> "search"
            "Play" -> "play"
            "Pause" -> "pause"
            "Previous" -> "prev"
            "Next" -> "next"
            "Heart-Filled" -> "hearted"
            "Heart" -> "noheart"
            "Queue" -> "queue"
            "More" -> "3dots_(nocircle)"
            "Shuffle" -> "ShuffleOFF"
            "Repeat" -> "RepeatOFF"
            else -> name
        }
    val context = LocalContext.current
    val surface = LocalMusicColors.current.control
    val picture =
        remember(name, color, surface) {
            runCatching {
                    var source =
                        context.assets.open("icons/$assetName.svg").bufferedReader().use {
                            it.readText()
                        }
                    source =
                        source
                            .replace(Regex("<use[^>]+fill=\"#FFFFFF\"[^>]*/?>"), "")
                            .replace("#000000", color.hex())
                            .replace("#3B3B3B", color.hex())
                            .replace("#FFFFFF", surface.hex())
                    SVG.getFromString(source)
                        .apply {
                            documentWidth = 32f
                            documentHeight = 32f
                        }
                        .renderToPicture()
                }
                .getOrNull()
        }
    Canvas(modifier) {
        picture?.let {
            drawIntoCanvas { canvas ->
                val native = canvas.nativeCanvas
                native.save()
                native.scale(size.width / 32f, size.height / 32f)
                native.drawPicture(it)
                native.restore()
            }
        }
    }
}

@Composable
fun PlayPauseIcon(playing: Boolean, modifier: Modifier = Modifier.size(32.dp)) {
    val context = LocalContext.current
    val colors = LocalMusicColors.current
    val endpoints = remember {
        val j =
            JSONObject(
                context.assets.open("transport-contours.json").bufferedReader().use {
                    it.readText()
                }
            )
        listOf("play", "pause").map { key ->
            val shapes = j.getJSONArray(key)
            (0 until shapes.length()).map { index ->
                val points = shapes.getJSONArray(index)
                FloatArray(points.length() * 2) { k ->
                    points.getJSONArray(k / 2).getDouble(k % 2).toFloat()
                }
            }
        }
    }
    val progress by
        animateFloatAsState(
            if (playing) 1f else 0f,
            tween(if (android.animation.ValueAnimator.areAnimatorsEnabled()) 360 else 0),
            label = "Sketch Play Pause",
        )
    Canvas(modifier) {
        scale(
            size.width / 32f,
            size.height / 32f,
            pivot = androidx.compose.ui.geometry.Offset.Zero,
        ) {
            val fill = Path()
            val outlines = mutableListOf<Path>()
            val seams = mutableListOf<Path>()
            for (index in 0..1) {
                val from = endpoints[0][index]
                val to = endpoints[1][index]
                fun p(k: Int) = from[k] + (to[k] - from[k]) * progress
                val full = Path().apply { moveTo(p(0), p(1)) }
                val outline = Path()
                val seam = Path()
                var x = p(0)
                var y = p(1)
                for (j in 0 until 16) {
                    val k = 2 + j * 6
                    full.cubicTo(p(k), p(k + 1), p(k + 2), p(k + 3), p(k + 4), p(k + 5))
                    val edge = if (j == (if (index == 0) 7 else 15)) seam else outline
                    if (j == 0 || j == 8 && index == 0 || j == (if (index == 0) 7 else 15))
                        edge.moveTo(x, y)
                    edge.cubicTo(p(k), p(k + 1), p(k + 2), p(k + 3), p(k + 4), p(k + 5))
                    x = p(k + 4)
                    y = p(k + 5)
                }
                full.close()
                fill.addPath(full)
                outlines += outline
                seams += seam
            }
            drawPath(
                fill,
                Brush.verticalGradient(listOf(colors.playbackTop, colors.playbackBottom), 0f, 32f),
            )
            val stroke = Stroke(3f, cap = StrokeCap.Round, join = StrokeJoin.Round)
            outlines.forEach { drawPath(it, colors.secondary, style = stroke) }
            seams.forEach { drawPath(it, colors.secondary.copy(alpha = progress), style = stroke) }
        }
    }
}

@Composable
fun Artwork(path: String?, modifier: Modifier = Modifier) {
    val colors = LocalMusicColors.current
    var edge by remember { androidx.compose.runtime.mutableIntStateOf(256) }
    val decoded by
        produceState<Pair<String?, androidx.compose.ui.graphics.ImageBitmap?>?>(null, path, edge) {
            value = path to
                withContext(Dispatchers.IO) {
                    runCatching { path?.let { CompanionArtwork.load(it, edge)?.asImageBitmap() } }
                        .getOrNull()
                }
        }
    val bitmap = decoded?.takeIf { it.first == path }?.second
    Box(
        modifier.aspectRatio(1f).onSizeChanged { size -> edge = listOf(96, 256, 512, 1024).firstOrNull { it >= maxOf(size.width, size.height) } ?: 1024 }.clip(RoundedCornerShape(16.dp)).background(colors.control),
        contentAlignment = Alignment.Center,
    ) {
        if (bitmap != null)
            Image(bitmap!!, null, Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        else SketchIcon("Album", Modifier.size(30.dp), colors.secondary)
    }
}

/** Private, content-addressed copies share bounded SDR decodes across shelves/player. */
internal object CompanionArtwork {
    private val cache = object : android.util.LruCache<String, android.graphics.Bitmap>(32 * 1024 * 1024) {
        override fun sizeOf(key: String, bitmap: android.graphics.Bitmap) = bitmap.allocationByteCount
    }
    @Synchronized fun load(path: String, edge: Int): android.graphics.Bitmap? {
        val file = java.io.File(path)
        val key = "$path:${file.length()}:${file.lastModified()}:$edge"
        cache.get(key)?.let { return it }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        var sample = 1
        while (maxOf(bounds.outWidth, bounds.outHeight) / sample > edge) sample *= 2
        val options = BitmapFactory.Options().apply {
            inSampleSize = sample
            inPreferredConfig = android.graphics.Bitmap.Config.ARGB_8888
            inPreferredColorSpace = ColorSpace.get(ColorSpace.Named.SRGB)
        }
        return BitmapFactory.decodeFile(path, options)?.apply {
            if (Build.VERSION.SDK_INT >= 34) gainmap = null
            cache.put(key, this)
        }
    }
}

@Composable
fun BeatingHeart(favorite: Boolean, playing: Boolean, modifier: Modifier = Modifier.size(16.dp)) {
    val pulse = remember { androidx.compose.animation.core.Animatable(1f) }
    val owner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    androidx.compose.runtime.LaunchedEffect(playing, owner) {
        pulse.snapTo(1f)
        if (playing && android.animation.ValueAnimator.areAnimatorsEnabled()) {
            owner.lifecycle.repeatOnLifecycle(androidx.lifecycle.Lifecycle.State.STARTED) {
                try {
                    while (true) {
                        pulse.animateTo(1.09f, tween(100))
                        pulse.animateTo(1f, tween(220))
                        kotlinx.coroutines.delay(513)
                    }
                } finally { pulse.snapTo(1f) }
            }
        }
    }
    SketchIcon(if (favorite) "Heart-Filled" else "Heart", modifier.graphicsLayer { scaleX = pulse.value; scaleY = pulse.value }, LocalMusicColors.current.accent)
}


private fun artworkSeed(path: String): Int? = runCatching {
    val options = BitmapFactory.Options().apply { inSampleSize = 8; inPreferredConfig = android.graphics.Bitmap.Config.ARGB_8888; if(Build.VERSION.SDK_INT >= 26) inPreferredColorSpace = ColorSpace.get(ColorSpace.Named.SRGB) }
    val bitmap = BitmapFactory.decodeFile(path, options) ?: return@runCatching null
    val buckets = mutableMapOf<Int, Pair<Int, Float>>()
    val hsv = FloatArray(3)
    try {
        for(y in 0 until bitmap.height step 3) for(x in 0 until bitmap.width step 3) {
            val pixel = bitmap.getPixel(x,y); android.graphics.Color.colorToHSV(pixel,hsv)
            if(hsv[1] < .15f || hsv[2] < .18f || hsv[2] > .97f) continue
            val bucket = (hsv[0]/15).toInt(); val old = buckets[bucket]
            buckets[bucket] = pixel to ((old?.second ?: 0f) + hsv[1])
        }
        buckets.values.maxByOrNull { it.second }?.first
    } finally { bitmap.recycle() }
}.getOrNull()

internal fun companionColors(seed: Int, dark: Boolean): MusicColors {
    val hsl = FloatArray(3); androidx.core.graphics.ColorUtils.colorToHSL(seed,hsl)
    fun tone(s: Float,l: Float) = Color(androidx.core.graphics.ColorUtils.HSLToColor(floatArrayOf(hsl[0],s,l)))
    val surface = if(dark) tone(.18f,.17f) else tone(.46f,.86f)
    val ink = if(dark) Color(0xffe7e9f2) else Color(0xff12202a)
    val accent = if(dark) tone(.65f,.75f) else tone(.65f,.32f)
    val secondary = Color(androidx.core.graphics.ColorUtils.blendARGB(ink.toArgb(),surface.toArgb(),.25f))
    val accentInk = listOf(Color.White, Color(0xff12202a), Color.Black).firstOrNull {
        androidx.core.graphics.ColorUtils.calculateContrast(it.toArgb(), accent.toArgb()) >= 4.5
    } ?: Color.Black
    return MusicColors(if(dark) tone(.15f,.12f) else tone(.62f,.68f),surface,if(dark) tone(.2f,.25f) else tone(.45f,.79f),ink,secondary,accent,accentInk,if(dark) tone(.4f,.79f) else tone(.35f,.4f),if(dark) tone(.4f,.60f) else tone(.4f,.25f))
}
