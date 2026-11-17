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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.3/CooklangImportFFI.xcframework.zip",
            checksum: "fa0f02e0f97724517cbdd1b3393f35f3fd3734dd623f8f1e6f1b693ebae41ff3"
        ),
        .target(
            name: "CooklangImportExtract",
            dependencies: ["CooklangImportExtractFFI"],
            path: "Sources/CooklangImportExtract"
        ),
        .binaryTarget(
            name: "CooklangImportExtractFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.3/CooklangImportExtractFFI.xcframework.zip",
            checksum: "fa5b025ff7263d084f7b8599cad79a22de314a449897272aa17c08f84a9524ce"
        ),
    ]
)
