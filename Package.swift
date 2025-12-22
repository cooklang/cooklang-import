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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.8.16/CooklangImportFFI.xcframework.zip",
            checksum: "d6b8c7e3b3503de5975618980ee0102a7b3b495b3a8b30be80f449bc5e1be7a0"
        ),
    ]
)
