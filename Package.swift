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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.0/CooklangImportFFI.xcframework.zip",
            checksum: "d43f1d242888814862fd5d99e908d7997e82b90a4506219a84d9d23fe14c6236"
        ),
        .target(
            name: "CooklangImportExtract",
            dependencies: ["CooklangImportExtractFFI"],
            path: "Sources/CooklangImportExtract"
        ),
        .binaryTarget(
            name: "CooklangImportExtractFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.0/CooklangImportExtractFFI.xcframework.zip",
            checksum: "edb2e30e8e4e8279b1cc2f001c5459b04fcbba08c931909106c0762a5e6f0674"
        ),
    ]
)
