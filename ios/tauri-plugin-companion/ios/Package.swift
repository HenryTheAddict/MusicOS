// swift-tools-version:5.7
import PackageDescription
let package=Package(
    name:"tauri-plugin-companion",
    platforms:[.iOS(.v16)],
    products:[.library(name:"tauri-plugin-companion",type:.static,targets:["tauri-plugin-companion"])],
    dependencies:[.package(name:"Tauri",path:"../.tauri/tauri-api"),.package(name:"MusicOSActivityShared",path:"../../activity-shared")],
    targets:[
        .target(name:"MusicOSBackgroundHooks",path:"BackgroundHooks",publicHeadersPath:"include"),
        .target(name:"tauri-plugin-companion",dependencies:[.byName(name:"Tauri"),.target(name:"MusicOSBackgroundHooks"),.product(name:"MusicOSActivityShared",package:"MusicOSActivityShared")],path:"Sources")
    ]
)
