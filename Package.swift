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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.1/CooklangImportFFI.xcframework.zip",
            checksum: "219698a7249714ae8738aa03a2afcfda60c0cc1f0f76451749c2130264a332a6"
        ),
        .target(
            name: "CooklangImportExtract",
            dependencies: ["CooklangImportExtractFFI"],
            path: "Sources/CooklangImportExtract"
        ),
        .binaryTarget(
            name: "CooklangImportExtractFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.1/CooklangImportExtractFFI.xcframework.zip",
            checksum: "ad7de0b73ee5e1326d95e4a6dc26b9b3548f39fc3e5436a0c1589f6e24ce3135"
        ),
    ]
)
