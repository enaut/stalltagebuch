#!/usr/bin/env bash
set -euo pipefail

# Wrapper Build für Android mit Custom MainActivity
# Schritte:
# 1) dx build --platform android (generiert Gradle-Projekt)
# 2) Custom-Dateien kopieren (MainActivity, Manifest, res/xml)
# 3) build.gradle.kts patchen (namespace, IDs, SDK)
# 4) ./gradlew assembleDebug
# 5) APK Pfad ausgeben

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DX_APP_DIR="$ROOT_DIR/target/dx/stalltagebuch/debug/android/app"
APP_SRC_MAIN="$DX_APP_DIR/app/src/main"
KOTLIN_DIR="$APP_SRC_MAIN/kotlin/dev/dioxus/main"
RES_XML_DIR="$APP_SRC_MAIN/res/xml"
GRADLE_FILE="$DX_APP_DIR/app/build.gradle.kts"
APK_OUT="$DX_APP_DIR/app/build/outputs/apk/debug/app-debug.apk"

# 1) Dioxus Build (vorher Altlasten entfernen, damit dx build nicht an einem alten BuildConfig-Alias scheitert)
echo "[1/5] Clean alte Android-Ausgabe und dx build --platform android"
rm -rf "$DX_APP_DIR" || true
dx build --platform android

# 2) Dateien kopieren
echo "[2/5] Kopiere Custom-Dateien"
mkdir -p "$KOTLIN_DIR" "$RES_XML_DIR"
cp "$ROOT_DIR/android/MainActivity.kt" "$KOTLIN_DIR/MainActivity.kt"
cp "$ROOT_DIR/android/AndroidManifest.xml" "$APP_SRC_MAIN/AndroidManifest.xml"
cp "$ROOT_DIR/android/res/xml/file_paths.xml" "$RES_XML_DIR/file_paths.xml"

# BuildConfig typealias, damit Logger.kt (dev.dioxus.main) auf das App-BuildConfig zugreifen kann
cat > "$KOTLIN_DIR/BuildConfig.kt" <<'EOF'
package dev.dioxus.main

import de.teilgedanken.stalltagebuch.BuildConfig as AppBuildConfig

typealias BuildConfig = AppBuildConfig
EOF

# Hinweis für zukünftige Wartung: Storage-Permissions
# - Ab SDK 33 (Tiramisu) wird READ_MEDIA_IMAGES verwendet
# - WRITE_EXTERNAL_STORAGE ist nur bis maxSdkVersion 28 wirksam
# - READ_EXTERNAL_STORAGE nur bis maxSdkVersion 32

# 3) Gradle patchen
if [[ -f "$GRADLE_FILE" ]]; then
  echo "[3/5] Patche build.gradle.kts"
  # Namespace und IDs
  sed -i 's/namespace="com\.example\.Stalltagebuch"/namespace="de.teilgedanken.stalltagebuch"/g' "$GRADLE_FILE"
  sed -i 's/applicationId = "com\.example\.Stalltagebuch"/applicationId = "de.teilgedanken.stalltagebuch"/g' "$GRADLE_FILE"
  
  # SDK Versionen
  sed -i 's/compileSdk = 33/compileSdk = 34/g' "$GRADLE_FILE"
  sed -i 's/targetSdk = 33/targetSdk = 34/g' "$GRADLE_FILE"
  sed -i 's/minSdk = 24/minSdk = 28/g' "$GRADLE_FILE"
  
  # Entferne kotlinOptions (deprecated)
  sed -i '/kotlinOptions {/,/}/d' "$GRADLE_FILE"
  
  # Füge JvmTarget Import hinzu
  sed -i '1i import org.jetbrains.kotlin.gradle.dsl.JvmTarget' "$GRADLE_FILE"
  
  # Füge Java 17 compileOptions und packaging nach defaultConfig hinzu
  sed -i '/defaultConfig {/,/}/a\    compileOptions {\n        sourceCompatibility = JavaVersion.VERSION_17\n        targetCompatibility = JavaVersion.VERSION_17\n    }\n    packaging {\n        jniLibs {\n            useLegacyPackaging = true\n        }\n    }\n    buildFeatures {\n        buildConfig = true\n    }' "$GRADLE_FILE"
  
  # Entferne alle java toolchain Blöcke (werden am Ende neu hinzugefügt)
  sed -i '/^java {/,/^}/d' "$GRADLE_FILE"
  
  # Entferne alte buildFeatures Blöcke innerhalb von android {} (außer dem neu hinzugefügten)
  # Dies wird durch die vorherige Zeile bereits teilweise erledigt
  
  # Füge einen sauberen java toolchain Block am Ende vor dependencies hinzu
  sed -i '/^dependencies {/i\java {\n    toolchain {\n        languageVersion = JavaLanguageVersion.of(17)\n    }\n}\n' "$GRADLE_FILE"
fi

# 4) Gradle Properties patchen (deprecated Einstellung entfernen)
GRADLE_PROPERTIES="$DX_APP_DIR/gradle.properties"
if [[ -f "$GRADLE_PROPERTIES" ]]; then
  echo "[4/5] Entferne deprecated BuildConfig-Einstellung aus gradle.properties"
  sed -i '/^android\.defaults\.buildfeatures\.buildconfig=/d' "$GRADLE_PROPERTIES"
fi

# 5) Gradle Build
echo "[5/5] Gradle assembleDebug"
cd "$DX_APP_DIR"
./gradlew clean assembleDebug --warning-mode all

# 6) Ergebnis
if [[ -f "$APK_OUT" ]]; then
  echo "[6/6] APK gebaut: $APK_OUT"
  echo "Hinweis: Prüfen auf MainActivity im APK: unzip -l \"$APK_OUT\" | grep -i mainactivity"
else
  echo "Fehler: APK nicht gefunden unter $APK_OUT" >&2
  exit 1
fi
