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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.8.14/CooklangImportFFI.xcframework.zip",
            checksum: "1cae4f488f5c6f10903fd3c0bee3dd12ae48699021ebd1701f082f16c131f4e1"
        ),
    ]
)
