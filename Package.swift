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
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.2/CooklangImportFFI.xcframework.zip",
            checksum: "d00dd83c22e86c11017f4e68a192205b5f531af0201c77df9615c8d8a031ceff"
        ),
        .target(
            name: "CooklangImportExtract",
            dependencies: ["CooklangImportExtractFFI"],
            path: "Sources/CooklangImportExtract"
        ),
        .binaryTarget(
            name: "CooklangImportExtractFFI",
            url: "https://github.com/cooklang/cooklang-import/releases/download/v0.13.2/CooklangImportExtractFFI.xcframework.zip",
            checksum: "bf09e69635451a3102945520279d2793df3e799bef5261daa3a2010caf2c4695"
        ),
    ]
)
