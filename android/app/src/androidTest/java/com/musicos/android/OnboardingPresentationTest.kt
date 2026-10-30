package com.musicos.android
import android.graphics.Bitmap
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.*
import org.junit.Assert.*
import org.junit.runner.RunWith
import java.io.File
@RunWith(AndroidJUnit4::class)
class OnboardingPresentationTest {
 @get:Rule val compose=createComposeRule()
 @Test fun phoneSetupKeepsActionsVisibleAndRetainsChoices() {
  val app=ApplicationProvider.getApplicationContext<MusicOSApp>();var mode by mutableStateOf("light");var short by mutableStateOf(false);var attempt=0;var result:Triple<String,String,Boolean>?=null
  compose.setContent { key(mode,short){Box(Modifier.width(360.dp).height(if(short)480.dp else 720.dp)){PhoneOnboarding(mode,"blue"){theme,colors,pair->attempt++;result=Triple(theme,colors,pair);attempt>1}}} }
  for(dark in listOf(false,true))for(small in listOf(false,true)) {
   compose.runOnIdle {mode=if(dark)"dark" else "light";short=small}
   compose.onNodeWithText("Your music, to go.").assertExists();compose.onNodeWithText("Continue").assertIsDisplayed()
   compose.waitForIdle();capture(app,"welcome-${mode}-${small}")
   compose.onNodeWithText("Continue").performClick();compose.onNodeWithText("Dark",substring=false).performClick();compose.onNodeWithText("From artwork").performClick();compose.onNodeWithText("Continue").assertIsDisplayed();compose.onNodeWithText("Continue").performClick();compose.onNodeWithText("Pair my Mac").assertIsDisplayed();compose.onNodeWithText("Explore first").assertIsDisplayed();capture(app,"pair-${mode}-${small}")
   if(!dark&&!small){compose.onNodeWithText("Explore first").performClick();compose.onNodeWithText("Couldn’t save setup. Please try again.").assertExists();compose.onNodeWithText("Explore first").performClick();compose.runOnIdle {assertEquals(Triple("dark","artwork",false),result)}}
  }
 }
 private fun capture(app:MusicOSApp,name:String){compose.waitForIdle();val out=File(app.getExternalFilesDir(null),"qa/onboarding-$name.png");out.parentFile!!.mkdirs();compose.onRoot().captureToImage().asAndroidBitmap().let {image->out.outputStream().use{image.compress(Bitmap.CompressFormat.PNG,100,it)}}}
}
