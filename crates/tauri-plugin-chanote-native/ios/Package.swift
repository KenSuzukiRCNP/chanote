// swift-tools-version:5.3
import PackageDescription

let package = Package(
  name: "tauri-plugin-chanote-native",
  platforms: [
    .macOS(.v10_13),
    .iOS(.v14),
  ],
  products: [
    .library(
      name: "tauri-plugin-chanote-native",
      type: .static,
      targets: ["tauri-plugin-chanote-native"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-chanote-native",
      dependencies: [
        .byName(name: "Tauri")
      ],
      path: "Sources")
  ]
)
