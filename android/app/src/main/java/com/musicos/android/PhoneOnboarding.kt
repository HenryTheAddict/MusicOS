package com.musicos.android

import android.content.Context
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import org.json.JSONObject

internal data class PhoneProduct(val name:String,val version:String,val onboardingVersion:Int,val title:String,val mode:String,val colors:String) {
    companion object {
        @Volatile private var cached:PhoneProduct?=null
        fun load(context:Context):PhoneProduct = cached ?: synchronized(this) {
            cached ?: context.assets.open("product.config.json").bufferedReader().use { reader ->
                val json=JSONObject(reader.readText());val defaults=json.getJSONObject("defaults").getJSONObject("appearance");val onboarding=json.getJSONObject("onboarding")
                PhoneProduct(json.getString("name"),json.getString("version"),onboarding.getInt("version"),onboarding.getString("phoneTitle"),defaults.getString("mode"),defaults.getString("source").takeIf { it=="artwork" }?:"blue").also { cached=it }
            }
        }
    }
}

@Composable
internal fun PhoneOnboarding(initialMode:String,initialColors:String,revisiting:Boolean=false,onFinish:suspend (String,String,Boolean)->Boolean) {
    val context=LocalContext.current
    val product=remember { PhoneProduct.load(context) }
    var step by rememberSaveable { mutableIntStateOf(0) }
    var mode by rememberSaveable { mutableStateOf(initialMode) }
    var source by rememberSaveable { mutableStateOf(initialColors) }
    var pending by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf(false) }
    val scope=rememberCoroutineScope()
    fun finish(pair:Boolean,defaults:Boolean=false) {
        if(pending)return
        pending=true;error=false
        scope.launch { try { error = !onFinish(if(defaults) initialMode else mode,if(defaults) initialColors else source,pair) } catch(_:Exception) { error=true } finally {pending=false} }
    }
    BackHandler(!pending) { if(step>0)step-- else finish(false,true) }
    val dark=mode=="dark" || mode=="system" && isSystemInDarkTheme()
    SideEffect {(context as? androidx.activity.ComponentActivity)?.let { activity -> androidx.core.view.WindowCompat.getInsetsController(activity.window,activity.window.decorView).apply {isAppearanceLightStatusBars=!dark;isAppearanceLightNavigationBars=!dark} }}
    MusicTheme(dark) {
        val colors=LocalMusicColors.current
        BoxWithConstraints(Modifier.fillMaxSize()) {
        val short=maxHeight<560.dp
        Surface(Modifier.fillMaxSize(),color=colors.surface,contentColor=colors.ink) {
            Column(Modifier.fillMaxSize().safeDrawingPadding().padding(horizontal=20.dp,vertical=12.dp),verticalArrangement=Arrangement.spacedBy(16.dp)) {
                Row(Modifier.fillMaxWidth(),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                    SketchIcon("Album",Modifier.size(28.dp));Text(product.name,fontSize=18.sp,fontWeight=FontWeight.ExtraBold,modifier=Modifier.weight(1f));Text("${step+1} of 3",fontSize=12.sp,color=colors.secondary)
                }
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(6.dp)) { repeat(3) { index -> Box(Modifier.weight(1f).height(4.dp).background(if(index<=step)colors.accent else colors.control,RoundedCornerShape(4.dp))) } }
                val entered=remember(step) { Animatable(0f) };val contentScroll=remember(step){ScrollState(0)}
                LaunchedEffect(step) { entered.animateTo(1f,tween(if(android.animation.ValueAnimator.areAnimatorsEnabled())180 else 0)) }
                Column(Modifier.weight(1f).fillMaxWidth().graphicsLayer { alpha=entered.value;translationY=if(android.animation.ValueAnimator.areAnimatorsEnabled())4.dp.toPx()*(1-entered.value) else 0f }.verticalScroll(contentScroll),verticalArrangement=Arrangement.spacedBy(16.dp)) {
                    if(step==0) {
                        Box(Modifier.fillMaxWidth().height(if(short)88.dp else 154.dp),contentAlignment=Alignment.Center) {
                            Box(Modifier.size(if(short)68.dp else 116.dp).graphicsLayer { rotationZ=-9f;translationX=-12.dp.toPx() }.background(colors.control,RoundedCornerShape(22.dp)))
                            Box(Modifier.size(if(short)68.dp else 116.dp).graphicsLayer { rotationZ=5f }.background(colors.accent,RoundedCornerShape(22.dp)),contentAlignment=Alignment.Center) { SketchIcon("Album",Modifier.size(if(short)42.dp else 66.dp),colors.accentInk) }
                        }
                    }
                    Text(if(step==0)product.title else if(step==1)"A familiar little space." else "Meet your Mac.",fontSize=if(short)28.sp else 32.sp,lineHeight=if(short)30.sp else 34.sp,fontWeight=FontWeight.ExtraBold)
                    Text(if(step==0)"Your library, artwork and collections. Ready on your phone, even when your Mac isn’t." else if(step==1)"Same ${product.name} feeling. Pick what feels right on this screen." else "Pair once over Wi-Fi. Choose copies on your Mac. Listen anywhere.",fontSize=15.sp,lineHeight=22.sp,color=colors.secondary)
                    if(step==0) {
                        if(!short) {
                            WelcomeRow("Library","Your collection, together","Albums and saved sequences come with you.")
                            WelcomeRow("Downloaded","Finished copies are yours to play","Keep listening when the Mac goes away.")
                        }
                    } else if(step==1) {
                        Text("Appearance",fontWeight=FontWeight.SemiBold)
                        FlowRow(horizontalArrangement=Arrangement.spacedBy(6.dp),verticalArrangement=Arrangement.spacedBy(6.dp)) { listOf("system" to "Follow system","light" to "Light","dark" to "Dark").forEach { (value,label) -> WelcomeChoice(label,mode==value) {mode=value} } }
                        Text("Colors",fontWeight=FontWeight.SemiBold)
                        FlowRow(horizontalArrangement=Arrangement.spacedBy(6.dp),verticalArrangement=Arrangement.spacedBy(6.dp)) { listOf("blue" to "${product.name} blue","artwork" to "From artwork").forEach { (value,label) -> WelcomeChoice(label,source==value) {source=value} } }
                        Text("Artwork colors follow the music once it’s on your phone.",fontSize=12.sp,color=colors.secondary)
                    } else {
                        WelcomeRow("sync","Same Wi-Fi, one pairing","Open Devices on your Mac, then scan its code here.",true)
                        WelcomeRow("Download","Choose your copies","Original files, or smaller Vorbis copies. Originals stay on the Mac.",true)
                        WelcomeRow("Heart-Filled","Keep your listening","Favorites stay with the phone when a copy is interrupted.",true)
                        Text("No Mac handy? Explore first and pair from Devices later.",fontSize=13.sp,lineHeight=19.sp,color=colors.secondary)
                    }
                }
                    if(error) Text("Couldn’t save setup. Please try again.",color=colors.ink,modifier=Modifier.border(1.dp,colors.accent,RoundedCornerShape(12.dp)).padding(12.dp))
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                    FilledTonalButton({if(step>0)step-- else finish(false,true)},Modifier.heightIn(min=48.dp),enabled=!pending,shape=RoundedCornerShape(14.dp)) {Text(if(step>0)"Back" else if(revisiting)"Keep my setup" else "Use defaults")}
                    Button({if(step<2)step++ else finish(true)},Modifier.weight(1f).heightIn(min=48.dp),enabled=!pending,shape=RoundedCornerShape(14.dp)) {Text(if(pending)"Saving…" else if(step==2)"Pair my Mac" else "Continue")}
                }
                if(step==2) FilledTonalButton({finish(false)},Modifier.fillMaxWidth().heightIn(min=44.dp),enabled=!pending,shape=RoundedCornerShape(14.dp)) {Text("Explore first")}
            }
        }
    }
    }
}

@Composable private fun WelcomeChoice(label:String,selected:Boolean,choose:()->Unit) {
    val colors=LocalMusicColors.current
    FilledTonalButton(choose,shape=RoundedCornerShape(14.dp),colors=ButtonDefaults.filledTonalButtonColors(containerColor=if(selected)colors.accent else colors.control,contentColor=if(selected)colors.accentInk else colors.ink),modifier=Modifier.heightIn(min=48.dp).semantics { this.selected=selected }) { Text(label,fontWeight=if(selected)FontWeight.Bold else FontWeight.Normal) }
}
@Composable private fun WelcomeRow(icon:String,title:String,description:String,compact:Boolean=false) {
    Row(Modifier.fillMaxWidth().background(if(compact)LocalMusicColors.current.surface else LocalMusicColors.current.control,RoundedCornerShape(16.dp)).padding(if(compact)8.dp else 14.dp),horizontalArrangement=Arrangement.spacedBy(12.dp),verticalAlignment=Alignment.CenterVertically) {
        SketchIcon(icon,Modifier.size(26.dp));Column(Modifier.weight(1f),verticalArrangement=Arrangement.spacedBy(3.dp)) {Text(title,fontWeight=FontWeight.SemiBold,fontSize=14.sp);Text(description,fontSize=12.sp,lineHeight=18.sp,color=LocalMusicColors.current.secondary)}
    }
}
