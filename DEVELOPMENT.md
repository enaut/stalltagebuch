# Development Guide

## Voraussetzungen

- **Rust** 1.83+ mit Android-Targets
- **Dioxus CLI** 0.7+
- **Android NDK** 26.1+
- **Android SDK** API 34
- **adb** (Android Debug Bridge)

## Setup

### Rust und Targets

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-linux-android aarch64-linux-android
```

### Dioxus CLI

```bash
curl -sSL http://dioxus.dev/install.sh | sh
```

### Android SDK/NDK

Setze \`ANDROID_HOME\` und \`NDK_HOME\` Umgebungsvariablen. Details siehe [Android Developer Docs](https://developer.android.com/studio/command-line/sdkmanager).

## Build

### Debug Build

```bash
./build_android.sh
```

**Was das Script macht:**
1. Clean alte Build-Artefakte
2. \`dx build --platform android\`
3. Kopiert custom \`MainActivity.kt\`, \`AndroidManifest.xml\`, \`file_paths.xml\`
4. Erstellt \`BuildConfig.kt\` Typealias
5. Patched \`build.gradle.kts\`
6. Gradle \`assembleDebug\`

**APK-Pfad:**
\`\`\`
target/dx/stalltagebuch/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
\`\`\`

### Installation

```bash
adb install -r target/dx/stalltagebuch/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

### Desktop Development (schneller)

```bash
dx serve --platform desktop
```

**Hinweis:** Camera/Gallery funktioniert nur auf Android.

## Android Emulator

### AVD erstellen

```bash
avdmanager create avd \\
    -n Medium_Phone_API_36 \\
    -k "system-images;android-36.1;google_apis_playstore;x86_64" \\
    -d "medium_phone_api_36"
```

### Emulator starten

```bash
emulator -avd Medium_Phone_API_36 &
adb wait-for-device
```

### Emulator verifizieren

```bash
adb devices
adb shell getprop ro.build.version.release
```

## Testing

### App starten

```bash
adb shell am start -n de.teilgedanken.stalltagebuch/dev.dioxus.main.MainActivity
```

### Live-Logs

```bash
adb logcat | grep -i stalltagebuch
adb logcat *:E | grep -i stalltagebuch  # Nur Errors
```

### Datenbank inspizieren

```bash
adb shell run-as de.teilgedanken.stalltagebuch cp /data/data/de.teilgedanken.stalltagebuch/files/stalltagebuch.db /sdcard/
adb pull /sdcard/stalltagebuch.db .
sqlite3 stalltagebuch.db
```

## Troubleshooting

### ClassNotFoundException: MainActivity

→ Nutze \`build_android.sh\` statt \`dx build\` direkt

### Camera/Gallery crashed

→ Prüfe Permissions in Settings: Apps → Stalltagebuch

### Build-Fehler

```bash
# NDK prüfen
ls \$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android*-clang

# ANDROID_HOME prüfen
echo \$ANDROID_HOME

# ADB neu starten
adb kill-server
adb start-server
```

## Development Workflow

```bash
# 1. Code ändern
vim src/components/home.rs

# 2. Rebuild & Install
./build_android.sh && adb install -r target/dx/stalltagebuch/debug/android/app/app/build/outputs/apk/debug/app-debug.apk

# 3. Logs beobachten
adb logcat | grep -i stalltagebuch
```

## Ressourcen

- **Dioxus Docs:** https://dioxuslabs.com/learn/0.7
- **Android Developer:** https://developer.android.com/
- **JNI Guide:** https://docs.rs/jni/latest/jni/
