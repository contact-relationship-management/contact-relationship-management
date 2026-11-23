// swift-tools-version: 6.0
// The iOS app's code (spec/mobile-apps, CRM-R111). CRMCore is Foundation only: the rules and the API client, tested with
// `swift test` on macOS. CRMMobileUI is the SwiftUI app drawn with Lily. App/ holds the Xcode project for the iOS target.
// Lily is fetched at a pinned commit by scripts/fetch-lily.sh (CRM-D28).
import PackageDescription

let package = Package(
    name: "CRMMobile",
    defaultLocalization: "en",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "CRMCore", targets: ["CRMCore"]),
        .library(name: "CRMMobileUI", targets: ["CRMMobileUI"]),
    ],
    dependencies: [
        .package(name: "LilyDesignSystem", path: "../vendor/lily-design-system/lily-design-system-swift"),
    ],
    targets: [
        .target(name: "CRMCore"),
        .target(
            name: "CRMMobileUI",
            dependencies: [
                "CRMCore",
                .product(name: "LilyUI", package: "LilyDesignSystem"),
                .product(name: "LilyThemes", package: "LilyDesignSystem"),
            ],
            resources: [.process("Resources")]
        ),
        .testTarget(name: "CRMCoreTests", dependencies: ["CRMCore"]),
        .testTarget(name: "CRMMobileUITests", dependencies: ["CRMMobileUI", "CRMCore"]),
    ],
    swiftLanguageModes: [.v6]
)
