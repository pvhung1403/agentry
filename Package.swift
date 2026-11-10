// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Loadout",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Loadout", targets: ["Loadout"]),
    ],
    targets: [
        // Pure Foundation: finds and removes what each agent loads. No UI, easy to test.
        .target(name: "LoadoutCore"),
        // SwiftUI app.
        .executableTarget(name: "Loadout", dependencies: ["LoadoutCore"]),
        .testTarget(name: "LoadoutCoreTests", dependencies: ["LoadoutCore"]),
    ]
)
