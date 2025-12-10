#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script to publish Android library to GitHub Packages (Maven)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
GROUP_ID="com.cooklang"
ARTIFACT_ID="cooklang-import"
REPO_OWNER="cooklang"
REPO_NAME="cooklang-import"

usage() {
    echo "Usage: $0 <version>"
    echo ""
    echo "Arguments:"
    echo "  version     The release version (e.g., 0.8.0)"
    echo ""
    echo "Environment variables:"
    echo "  GITHUB_TOKEN    Required for publishing (or use GITHUB_ACTOR with token)"
    echo "  GITHUB_ACTOR    GitHub username (defaults to repo owner)"
    echo ""
    echo "Examples:"
    echo "  GITHUB_TOKEN=ghp_xxx $0 0.8.0"
    exit 1
}

setup_gradle_project() {
    local version="$1"
    local android_dir="${PROJECT_ROOT}/target/android/cooklang-import-android"

    if [[ ! -d "$android_dir" ]]; then
        echo -e "${RED}Error: Android build directory not found at ${android_dir}${NC}"
        echo -e "${RED}Run scripts/build-android.sh first${NC}"
        exit 1
    fi

    echo -e "${YELLOW}Setting up Gradle project for publishing...${NC}"

    # Create settings.gradle.kts
    cat > "${android_dir}/settings.gradle.kts" << EOF
rootProject.name = "cooklang-import-android"
EOF

    # Update build.gradle.kts with publishing configuration
    cat > "${android_dir}/build.gradle.kts" << EOF
plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
}

group = "${GROUP_ID}"
version = "${version}"

android {
    namespace = "${GROUP_ID}.import_lib"
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
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "${GROUP_ID}"
            artifactId = "${ARTIFACT_ID}"
            version = "${version}"

            afterEvaluate {
                from(components["release"])
            }

            pom {
                name.set("CooklangImport")
                description.set("Android library for importing recipes into Cooklang format")
                url.set("https://github.com/${REPO_OWNER}/${REPO_NAME}")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                developers {
                    developer {
                        id.set("cooklang")
                        name.set("Cooklang Team")
                        email.set("hello@cooklang.org")
                    }
                }

                scm {
                    connection.set("scm:git:git://github.com/${REPO_OWNER}/${REPO_NAME}.git")
                    developerConnection.set("scm:git:ssh://github.com/${REPO_OWNER}/${REPO_NAME}.git")
                    url.set("https://github.com/${REPO_OWNER}/${REPO_NAME}")
                }
            }
        }
    }

    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/${REPO_OWNER}/${REPO_NAME}")
            credentials {
                username = System.getenv("GITHUB_ACTOR") ?: "${REPO_OWNER}"
                password = System.getenv("GITHUB_TOKEN") ?: ""
            }
        }
    }
}
EOF

    # Create gradle.properties
    cat > "${android_dir}/gradle.properties" << EOF
android.useAndroidX=true
kotlin.code.style=official
org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.nonTransitiveRClass=true
EOF

    # Create local.properties if ANDROID_HOME is set
    if [[ -n "${ANDROID_HOME:-}" ]]; then
        echo "sdk.dir=${ANDROID_HOME}" > "${android_dir}/local.properties"
    elif [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
        echo "sdk.dir=${ANDROID_SDK_ROOT}" > "${android_dir}/local.properties"
    fi

    echo -e "${GREEN}Gradle project configured${NC}"
}

setup_gradle_wrapper() {
    local android_dir="${PROJECT_ROOT}/target/android/cooklang-import-android"

    echo -e "${YELLOW}Setting up Gradle wrapper...${NC}"

    cd "$android_dir"

    # Check if gradle is available
    if command -v gradle &> /dev/null; then
        gradle wrapper --gradle-version 8.5
    else
        # Download and set up gradle wrapper manually
        mkdir -p gradle/wrapper

        # Download gradle-wrapper.jar
        curl -fsSL "https://raw.githubusercontent.com/gradle/gradle/v8.5.0/gradle/wrapper/gradle-wrapper.jar" \
            -o gradle/wrapper/gradle-wrapper.jar

        # Create gradle-wrapper.properties
        cat > gradle/wrapper/gradle-wrapper.properties << 'EOF'
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-8.5-bin.zip
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
EOF

        # Create gradlew script
        cat > gradlew << 'GRADLEW_EOF'
#!/bin/sh
# Gradle wrapper script

# Determine the Java command to use to start the JVM
if [ -n "$JAVA_HOME" ] ; then
    JAVACMD="$JAVA_HOME/bin/java"
else
    JAVACMD="java"
fi

# Escape application args
save () {
    for i do printf %s\\n "$i" | sed "s/'/'\\\\''/g;1s/^/'/;\$s/\$/' \\\\/" ; done
    echo " "
}
APP_ARGS=$(save "$@")

exec "$JAVACMD" -jar "$0/../gradle/wrapper/gradle-wrapper.jar" "$@"
GRADLEW_EOF
        chmod +x gradlew
    fi

    cd "$PROJECT_ROOT"
    echo -e "${GREEN}Gradle wrapper configured${NC}"
}

publish_to_github_packages() {
    local android_dir="${PROJECT_ROOT}/target/android/cooklang-import-android"

    echo -e "${YELLOW}Publishing to GitHub Packages...${NC}"

    cd "$android_dir"

    # Run gradle publish
    ./gradlew publish --no-daemon

    cd "$PROJECT_ROOT"
    echo -e "${GREEN}Published to GitHub Packages${NC}"
}

main() {
    if [[ $# -lt 1 ]]; then
        usage
    fi

    local version="$1"

    # Remove 'v' prefix if present
    version="${version#v}"

    # Check for required environment variables
    if [[ -z "${GITHUB_TOKEN:-}" ]]; then
        echo -e "${RED}Error: GITHUB_TOKEN environment variable is required${NC}"
        usage
    fi

    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Android Package Publisher             ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Version: ${version}"
    echo "Group ID: ${GROUP_ID}"
    echo "Artifact ID: ${ARTIFACT_ID}"
    echo "Repository: ${REPO_OWNER}/${REPO_NAME}"
    echo ""

    setup_gradle_project "$version"
    setup_gradle_wrapper
    publish_to_github_packages

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Publication Complete!                 ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "To use this package in your Android project, add:"
    echo ""
    echo "  In settings.gradle.kts:"
    echo "    dependencyResolutionManagement {"
    echo "        repositories {"
    echo "            maven {"
    echo "                url = uri(\"https://maven.pkg.github.com/${REPO_OWNER}/${REPO_NAME}\")"
    echo "                credentials {"
    echo "                    username = project.findProperty(\"gpr.user\") ?: System.getenv(\"GITHUB_ACTOR\")"
    echo "                    password = project.findProperty(\"gpr.key\") ?: System.getenv(\"GITHUB_TOKEN\")"
    echo "                }"
    echo "            }"
    echo "        }"
    echo "    }"
    echo ""
    echo "  In build.gradle.kts:"
    echo "    implementation(\"${GROUP_ID}:${ARTIFACT_ID}:${version}\")"
}

main "$@"
