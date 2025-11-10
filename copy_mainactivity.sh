#!/bin/bash
# Post-build script to copy custom files and patch package names

MAIN_SOURCE="./android/MainActivity.kt"
BUILD_CONFIG_SOURCE="./android/BuildConfig.kt"
MANIFEST_SOURCE="./android/AndroidManifest.xml"
FILE_PATHS_SOURCE="./android/res/xml/file_paths.xml"
GRADLE_SOURCE="./android/build.gradle.kts"
TARGET_DEBUG="./target/dx/stalltagebuch/debug/android/app/app/src/main/kotlin/dev/dioxus/main/MainActivity.kt"
TARGET_RELEASE="./target/dx/stalltagebuch/release/android/app/app/src/main/kotlin/dev/dioxus/main/MainActivity.kt"
BUILD_CONFIG_DEBUG="./target/dx/stalltagebuch/debug/android/app/app/src/main/kotlin/dev/dioxus/main/BuildConfig.kt"
BUILD_CONFIG_RELEASE="./target/dx/stalltagebuch/release/android/app/app/src/main/kotlin/dev/dioxus/main/BuildConfig.kt"
MANIFEST_DEBUG="./target/dx/stalltagebuch/debug/android/app/app/src/main/AndroidManifest.xml"
MANIFEST_RELEASE="./target/dx/stalltagebuch/release/android/app/app/src/main/AndroidManifest.xml"
FILE_PATHS_DEBUG="./target/dx/stalltagebuch/debug/android/app/app/src/main/res/xml/file_paths.xml"
FILE_PATHS_RELEASE="./target/dx/stalltagebuch/release/android/app/app/src/main/res/xml/file_paths.xml"
GRADLE_DEBUG="./target/dx/stalltagebuch/debug/android/app/app/build.gradle.kts"
GRADLE_RELEASE="./target/dx/stalltagebuch/release/android/app/app/build.gradle.kts"

if [ -f "$TARGET_DEBUG" ]; then
    echo "Patching build.gradle.kts for debug build..."
    if [ -f "$GRADLE_DEBUG" ]; then
        sed -i 's/namespace="com\.example\.Stalltagebuch"/namespace="de.teilgedanken.stalltagebuch"/g' "$GRADLE_DEBUG"
        sed -i 's/applicationId = "com\.example\.Stalltagebuch"/applicationId = "de.teilgedanken.stalltagebuch"/g' "$GRADLE_DEBUG"
        sed -i 's/compileSdk = 33/compileSdk = 34/g' "$GRADLE_DEBUG"
        sed -i 's/targetSdk = 33/targetSdk = 34/g' "$GRADLE_DEBUG"
        sed -i 's/minSdk = 24/minSdk = 28/g' "$GRADLE_DEBUG"
        echo "✓ build.gradle.kts patched for debug"
    fi
    
    echo "Copying MainActivity.kt to debug build..."
    cp "$MAIN_SOURCE" "$TARGET_DEBUG"
    # Touch the file to force recompilation
    touch "$TARGET_DEBUG"
    echo "✓ MainActivity.kt copied to debug"
    
    echo "Copying BuildConfig.kt to debug build..."
    cp "$BUILD_CONFIG_SOURCE" "$BUILD_CONFIG_DEBUG"
    # Patch BuildConfig to use correct package
    sed -i 's/com\.example\.Stalltagebuch/de.teilgedanken.stalltagebuch/g' "$BUILD_CONFIG_DEBUG"
    touch "$BUILD_CONFIG_DEBUG"
    echo "✓ BuildConfig.kt copied and patched for debug"
    
    echo "Copying AndroidManifest.xml to debug build..."
    cp "$MANIFEST_SOURCE" "$MANIFEST_DEBUG"
    echo "✓ AndroidManifest.xml copied to debug"
    
    echo "Copying file_paths.xml to debug build..."
    mkdir -p "$(dirname "$FILE_PATHS_DEBUG")"
    cp "$FILE_PATHS_SOURCE" "$FILE_PATHS_DEBUG"
    echo "✓ file_paths.xml copied to debug"
    
    # Clean Kotlin compilation cache to force rebuild
    echo "Cleaning Kotlin build cache for debug..."
    rm -rf "./target/dx/stalltagebuch/debug/android/app/app/build/tmp/kotlin-classes/debug/dev/dioxus/main/MainActivity.class"
    rm -rf "./target/dx/stalltagebuch/debug/android/app/app/build/tmp/kotlin-classes/debug/dev/dioxus/main/BuildConfig.class"
    echo "✓ Build cache cleaned for debug"
fi

if [ -f "$TARGET_RELEASE" ]; then
    echo "Patching build.gradle.kts for release build..."
    if [ -f "$GRADLE_RELEASE" ]; then
        sed -i 's/namespace="com\.example\.Stalltagebuch"/namespace="de.teilgedanken.stalltagebuch"/g' "$GRADLE_RELEASE"
        sed -i 's/applicationId = "com\.example\.Stalltagebuch"/applicationId = "de.teilgedanken.stalltagebuch"/g' "$GRADLE_RELEASE"
        sed -i 's/compileSdk = 33/compileSdk = 34/g' "$GRADLE_RELEASE"
        sed -i 's/targetSdk = 33/targetSdk = 34/g' "$GRADLE_RELEASE"
        sed -i 's/minSdk = 24/minSdk = 28/g' "$GRADLE_RELEASE"
        echo "✓ build.gradle.kts patched for release"
    fi
    
    echo "Copying MainActivity.kt to release build..."
    cp "$MAIN_SOURCE" "$TARGET_RELEASE"
    touch "$TARGET_RELEASE"
    echo "✓ MainActivity.kt copied to release"
    
    echo "Copying BuildConfig.kt to release build..."
    cp "$BUILD_CONFIG_SOURCE" "$BUILD_CONFIG_RELEASE"
    # Patch BuildConfig to use correct package
    sed -i 's/com\.example\.Stalltagebuch/de.teilgedanken.stalltagebuch/g' "$BUILD_CONFIG_RELEASE"
    touch "$BUILD_CONFIG_RELEASE"
    echo "✓ BuildConfig.kt copied and patched for release"
    
    echo "Copying AndroidManifest.xml to release build..."
    cp "$MANIFEST_SOURCE" "$MANIFEST_RELEASE"
    echo "✓ AndroidManifest.xml copied to release"
    
    echo "Copying file_paths.xml to release build..."
    mkdir -p "$(dirname "$FILE_PATHS_RELEASE")"
    cp "$FILE_PATHS_SOURCE" "$FILE_PATHS_RELEASE"
    echo "✓ file_paths.xml copied to release"
    
    # Clean Kotlin compilation cache to force rebuild
    echo "Cleaning Kotlin build cache for release..."
    rm -rf "./target/dx/stalltagebuch/release/android/app/app/build/tmp/kotlin-classes/release/dev/dioxus/main/MainActivity.class"
    rm -rf "./target/dx/stalltagebuch/release/android/app/app/build/tmp/kotlin-classes/release/dev/dioxus/main/BuildConfig.class"
    echo "✓ Build cache cleaned for release"
fi
