# Verdure Android App

This is the Tauri-based Android application for Verdure, your local AI conductor.

## What is Verdure?

Verdure helps you discover, install, and run the right open-source AI tools on your own device. No cloud required. Your data stays with you.

## Building the Android APK

### Prerequisites

1. **Rust** (1.90 or newer)
   ```bash
   rustup update stable
   ```

2. **Android SDK and NDK**
   - Install Android SDK command line tools
   - Set up environment variables:
     ```bash
     export ANDROID_HOME=~/Android/Sdk
     export ANDROID_NDK_HOME=~/Android/Sdk/ndk/27.0.12077973
     export NDK_HOME=$ANDROID_NDK_HOME
     export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools
     ```

3. **Tauri CLI**
   ```bash
   cargo install tauri-cli --locked
   ```

### Build Steps

1. **Initialize Android platform** (first time only):
   ```bash
   cd app
   cargo tauri android init
   ```

2. **Build the APK**:
   ```bash
   cargo tauri android build --apk
   ```

   The unsigned APK will be generated at:
   ```
   app/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
   ```

## Installing on Your Phone

### Option 1: Using ADB (Android Debug Bridge)

1. **Enable Developer Options** on your Android phone:
   - Go to Settings > About Phone
   - Tap "Build Number" 7 times
   - Go back to Settings > Developer Options
   - Enable "USB Debugging"

2. **Connect your phone** via USB to your computer

3. **Install the APK**:
   ```bash
   adb install app-universal-release-unsigned.apk
   ```

   Or if you have an older version installed:
   ```bash
   adb install -r app-universal-release-unsigned.apk
   ```

### Option 2: Direct Transfer

1. Copy the APK file to your phone (via USB, cloud storage, etc.)
2. Open the APK file on your phone
3. Allow installation from unknown sources if prompted
4. Install the app

## Development

To run the app in development mode on an Android emulator or connected device:

```bash
cd app
cargo tauri android dev
```

## Architecture

This app is built with:
- **Backend**: Rust (Tauri)
- **Frontend**: HTML/CSS/JavaScript
- **Platform**: Android (with iOS support planned)

The app follows the architecture outlined in `knowledge/17-build-plan.md`:
- Reuses the decider logic from `website/lib`
- Provides one-click installation of OSS AI tools
- Runs everything locally for privacy

## Current Status

This is the MVP scaffolding. The app currently shows:
- A beautiful landing page explaining Verdure's purpose
- The core UI framework ready for the decider integration

Next steps (from the build plan):
1. Integrate the intent cards and decision logic from `website/lib`
2. Add the tool catalog and porter functionality
3. Implement one-click installation for the first tool (Buzz transcription)

## License

See the main repository LICENSE file.
