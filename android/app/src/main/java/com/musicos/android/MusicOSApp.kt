package com.musicos.android

import android.app.Application

class MusicOSApp : Application() {
    val repository: MusicRepository by lazy { MusicRepository(this) }
}
