# Verdure Android APK - Quick Installation Guide

## 📱 Installing the APK on Your Phone

The Android APK for Verdure has been successfully built! Here's how to install it on your phone for testing.

### Generated APK Location

The APK is located at:
```
app/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

Size: ~36 MB

### Method 1: Install via ADB (Recommended)

**Prerequisites:**
- USB cable to connect phone to laptop
- ADB (Android Debug Bridge) installed on laptop
- Developer Options enabled on phone

**Steps:**

1. **Enable Developer Options on your phone:**
   - Go to Settings → About Phone
   - Tap "Build Number" 7 times rapidly
   - You'll see "You are now a developer!"

2. **Enable USB Debugging:**
   - Go to Settings → Developer Options
   - Turn on "USB Debugging"
   - You may also want to enable "Install via USB"

3. **Connect your phone to your laptop via USB**
   - When prompted on phone, allow USB debugging from this computer

4. **Verify connection:**
   ```bash
   adb devices
   ```
   You should see your device listed.

5. **Install the APK:**
   ```bash
   cd /workspace/app/src-tauri/gen/android/app/build/outputs/apk/universal/release
   adb install app-universal-release-unsigned.apk
   ```

   If you already have the app installed:
   ```bash
   adb install -r app-universal-release-unsigned.apk
   ```

6. **Launch the app** from your phone's app drawer!

### Method 2: Direct Transfer (No USB Debugging Needed)

1. **Copy the APK to your phone:**
   - Connect phone via USB in File Transfer mode, OR
   - Upload to Google Drive/Dropbox and download on phone, OR
   - Send via Bluetooth/nearby share, OR
   - Email it to yourself

2. **On your phone:**
   - Open the Files app
   - Navigate to where you saved the APK
   - Tap on the APK file

3. **Install:**
   - Android may prompt "Install unknown apps"
   - Allow your file manager/browser to install apps
   - Tap "Install"

4. **Done!** The Verdure app should now be on your home screen

### Troubleshooting

**"App not installed" error:**
- Make sure you've allowed installation from unknown sources
- Try uninstalling any previous version first
- Check that you have enough storage space

**USB Debugging issues:**
- Try revoking USB debugging authorizations (in Developer Options) and reconnecting
- Use a different USB cable (some cables are charge-only)
- Make sure USB is in "File Transfer" or "MTP" mode, not "Charging only"

**ADB not found:**
```bash
# On Ubuntu/Debian:
sudo apt-get install android-tools-adb

# On macOS:
brew install android-platform-tools

# Or download from Android SDK Platform Tools
```

### What You'll See

The app displays:
- 🌿 **Verdure branding** with beautiful gradient design
- A welcoming landing page explaining Verdure's purpose
- Feature highlights:
  - 🎯 Smart Recommendations
  - ⚡ One-Click Install
  - 🔒 Privacy First
  - 🆓 Free & Open Source

This is the MVP foundation. The full decider and tool porter functionality will be integrated next!

### Rebuilding

To rebuild the APK with changes:
```bash
cd /workspace/app
cargo tauri android build --apk
```

The new APK will overwrite the previous one at the same location.

---

## 🛠️ Technical Details

- **Framework**: Tauri 2.x (Rust backend + web frontend)
- **Bundle ID**: com.verdure.app
- **Target SDK**: Android 34
- **Architecture**: Universal (supports all ARM and x86 devices)
- **Signing**: Unsigned (development build)

For more details, see `app/README.md`
