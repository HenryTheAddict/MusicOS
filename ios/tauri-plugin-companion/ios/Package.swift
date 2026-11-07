// swift-tools-version:5.5

import PackageDescription

let package = Package(
    name: "tauri-plugin-companion",
    platforms: [
        .macOS(.v10_13),
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "tauri-plugin-companion",
            type: .static,
            targets: ["tauri-plugin-companion"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "tauri-plugin-companion",
            dependencies: [
                .byName(name: "Tauri")
            ],
            path: "Sources")
    ]
)
