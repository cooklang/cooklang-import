// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "CooklangImport",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "CooklangImport",
            targets: ["CooklangImport", "CooklangImportFFI"]
        ),
    ],
    targets: [
        .target(
            name: "CooklangImport",
            dependencies: ["CooklangImportFFI"],
            path: "Sources/CooklangImport"
        ),
        .binaryTarget(
            name: "CooklangImportFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.8.3/CooklangImportFFI.xcframework.zip",
            checksum: "56c8f87b73e8855a0bd8f1964a075e6fc9c3b04bb9bbae04124da84e8f1dca58"
        ),
    ]
)
