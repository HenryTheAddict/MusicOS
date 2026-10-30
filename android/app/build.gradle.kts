import groovy.json.JsonSlurper
import java.io.File
import java.util.Properties
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

val product = JsonSlurper().parse(rootProject.file("../product.config.json")) as Map<*,*>
abstract class ProductAssets: DefaultTask() {
    @get:InputFile abstract val configuration: RegularFileProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
    @TaskAction fun generate() { val folder=outputDirectory.get().asFile;folder.mkdirs();configuration.get().asFile.copyTo(File(folder,"product.config.json"),overwrite=true) }
}
val productAssets=tasks.register<ProductAssets>("prepareProductAssets") { configuration.set(rootProject.file("../product.config.json")); outputDirectory.set(layout.buildDirectory.dir("generated/product-assets")) }
androidComponents.onVariants { variant -> variant.sources.assets?.addGeneratedSourceDirectory(productAssets,ProductAssets::outputDirectory) }

android {
    namespace = "com.musicos.android"
    compileSdk = 37
    defaultConfig {
        applicationId = "com.musicos.android"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = product["version"] as String
        resValue("string", "app_name", product["name"] as String)
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    buildFeatures { compose = true; resValues = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    val privateSigning =
        System.getenv("MUSICOS_ANDROID_SIGNING")?.let { path ->
            Properties().apply { File(path).inputStream().use { load(it) } }
        }
    signingConfigs {
        if (privateSigning != null)
            create("musicos") {
                storeFile = file(privateSigning.getProperty("storeFile"))
                storePassword = privateSigning.getProperty("storePassword")
                keyAlias = privateSigning.getProperty("keyAlias")
                keyPassword = privateSigning.getProperty("keyPassword")
            }
    }
    buildTypes {
        release {
            isMinifyEnabled = false
            if (privateSigning != null) signingConfig = signingConfigs.getByName("musicos")
        }
    }
    testOptions { unitTests.isIncludeAndroidResources = true }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2025.08.01"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.2")
    implementation("androidx.media3:media3-exoplayer:1.11.1")
    implementation("androidx.media3:media3-session:1.11.1")
    implementation("androidx.work:work-runtime-ktx:2.10.2")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.caverock:androidsvg-aar:1.4")
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("androidx.test:core:1.7.0")
    testImplementation("org.robolectric:robolectric:4.15.1")
    testImplementation("com.squareup.okhttp3:mockwebserver:4.12.0")
    testImplementation("com.squareup.okhttp3:okhttp-tls:4.12.0")
    androidTestImplementation(platform("androidx.compose:compose-bom:2025.08.01"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.test:rules:1.7.0")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.7.0")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
