#!/bin/bash
set -euo pipefail

# Turns an Android library module into a Gradle build with Maven publishing, and
# runs it. The release workflow publishes both Android SDKs with it:
#
#   full      org.cooklang:cooklang-import           target/android/cooklang-import-android
#   extract   org.cooklang:cooklang-import-extract   target/android-extract/cooklang-import-extract-android
#
# Usage: publish-android-module.sh <full|extract> <module-dir> <version> [gradle-task]
#
# The task is `publish` (GitHub Packages; needs GITHUB_ACTOR and GITHUB_TOKEN) unless
# another is given: `publishToMavenLocal` puts the same artifact in ~/.m2, which is
# how to try a build in an app before it is released.
#
# Needs `gradle` on the PATH (any recent one; it only writes the 8.5 wrapper), a JDK
# that Gradle 8.5 runs on (17 or 21) and the Android SDK (ANDROID_HOME).

if [[ $# -lt 3 ]]; then
    sed -n 's/^# Usage: /usage: /p' "$0" >&2
    exit 1
fi

VARIANT="$1"
MODULE_DIR="$2"
VERSION="${3#v}"
GRADLE_TASK="${4:-publish}"

case "$VARIANT" in
    full)
        ARTIFACT_ID="cooklang-import"
        NAMESPACE="org.cooklang.cooklang_import"
        PROJECT_NAME="cooklang-import-android"
        POM_NAME="CooklangImport"
        POM_DESCRIPTION="Android library for importing recipes into Cooklang format"
        # Its bindings have suspend functions.
        DEPENDENCIES='    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")'
        ;;
    extract)
        ARTIFACT_ID="cooklang-import-extract"
        NAMESPACE="org.cooklang.cooklang_import_extract"
        PROJECT_NAME="cooklang-import-extract-android"
        POM_NAME="CooklangImportExtract"
        POM_DESCRIPTION="Android library for extracting a recipe from a page's HTML, without the network stack of cooklang-import"
        # Its bindings are synchronous: JNA is all they import.
        DEPENDENCIES='    implementation("net.java.dev.jna:jna:5.14.0@aar")'
        ;;
    *)
        echo "error: unknown variant '${VARIANT}' (full or extract)" >&2
        exit 1
        ;;
esac

if [[ ! -f "${MODULE_DIR}/src/main/AndroidManifest.xml" ]]; then
    echo "error: ${MODULE_DIR} is not an Android library module" >&2
    exit 1
fi

cat > "${MODULE_DIR}/settings.gradle.kts" << EOF
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "${PROJECT_NAME}"
EOF

cat > "${MODULE_DIR}/build.gradle.kts" << EOF
plugins {
    id("com.android.library") version "8.2.0"
    id("org.jetbrains.kotlin.android") version "1.9.22"
    id("maven-publish")
}

group = "org.cooklang"
version = "${VERSION}"

android {
    namespace = "${NAMESPACE}"
    compileSdk = 34

    defaultConfig {
        minSdk = 21
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")

        aarMetadata {
            minCompileSdk = 21
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
${DEPENDENCIES}
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "org.cooklang"
            artifactId = "${ARTIFACT_ID}"
            version = "${VERSION}"

            afterEvaluate {
                from(components["release"])
            }

            pom {
                name.set("${POM_NAME}")
                description.set("${POM_DESCRIPTION}")
                url.set("https://github.com/cooklang/cooklang-import")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                scm {
                    connection.set("scm:git:git://github.com/cooklang/cooklang-import.git")
                    developerConnection.set("scm:git:ssh://github.com/cooklang/cooklang-import.git")
                    url.set("https://github.com/cooklang/cooklang-import")
                }
            }
        }
    }

    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/cooklang/cooklang-import")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
EOF

cat > "${MODULE_DIR}/gradle.properties" << 'EOF'
android.useAndroidX=true
kotlin.code.style=official
org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.nonTransitiveRClass=true
EOF

cd "$MODULE_DIR"
gradle wrapper --gradle-version 8.5
./gradlew "$GRADLE_TASK" --no-daemon
