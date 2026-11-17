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
        // extractRecipeFromHtml only: no network, no LLM. Link this or
        // CooklangImport, never both.
        .library(
            name: "CooklangImportExtract",
            targets: ["CooklangImportExtract", "CooklangImportExtractFFI"]
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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.4/CooklangImportFFI.xcframework.zip",
            checksum: "b7cede06b6eddb13d391cc116d56dffe4f1ac7af8b10fbe2c18e9cd2266ffff9"
        ),
        .target(
            name: "CooklangImportExtract",
            dependencies: ["CooklangImportExtractFFI"],
            path: "Sources/CooklangImportExtract"
        ),
        .binaryTarget(
            name: "CooklangImportExtractFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.4/CooklangImportExtractFFI.xcframework.zip",
            checksum: "ba2f06d8218b9003075261eecb67595022f961a75622693d1a3304ace8ccfb68"
        ),
    ]
)
