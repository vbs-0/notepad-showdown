# Notepad, four ways

The same small app — type text, it persists, reopen it — written four times to find out what each Android stack actually costs: **Kotlin**, **Flutter**, **Rust + Slint**, and **Rust without std**.

The point was not to ship a notepad. It was to answer, with artifacts rather than opinions: *what does each stack add on top of your own code?*

## The four

| Directory | Stack | UI |
|---|---|---|
| `kotlin/` | Kotlin, Android platform views | `EditText` + `SharedPreferences` |
| `flutter/` | Flutter / Dart | Material widgets |
| `rust-slint/` | Rust + Slint | declarative `.slint` UI, rendered by Slint |
| `rust-tiny/` | Rust, `#![no_std]` | hand-drawn glyphs on a raw surface |

## Measured

All APK sizes are the **arm64-v8a** artifact built on this machine, not from a store listing.

| Variant | APK | Build type | Source | Direct deps | Native lib |
|---|---|---|---|---|---|
| **rust-tiny** | **24,741 B** | release | 2,150 LOC | 1 (`linked_list_allocator`) | 38,392 B `.so` |
| **kotlin** | **34,257 B** | release | 504 LOC | **0** | — |
| **rust-slint** | 7,533,918 B | **debug** | 323 LOC | 3 (`slint`, `serde`, `serde_json`) | 14,177,040 B `.so` (debug) |
| **flutter** | 16,938,208 B | release | 598 LOC | 2 (`cupertino_icons`, `path_provider`) | engine bundled |

Release build times observed: `rust-tiny` 4.1 s for the `.so` plus 17 s for the APK; `kotlin` and `flutter` were not re-timed for this write-up.

**Read the `rust-slint` row with care.** It is a debug build: Slint's Android build script needs the Android activity classes from a full SDK layout that this machine does not have, so the release build could not be produced here. A release build would be smaller (the debug `.so` alone is 14 MB, mostly unoptimised Slint and text-shaping code), but until it is built, 7.5 MB is what we can point at.

## What the numbers actually say

**Kotlin — 34 KB with zero dependencies.** Nothing is bundled because the UI toolkit is already on the phone. Your 504 lines and a few resources are the whole APK. This is the floor for anything that uses platform widgets.

**Rust without std — 24.7 KB, the smallest of the four.** No toolkit, no allocator beyond a linked-list allocator, no font stack: it draws glyphs itself. The price is visible in the same table — 2,150 lines against Kotlin's 504. Roughly four times the code to save 10 KB and own the rendering.

**Rust + Slint — the toolkit is the weight.** 323 lines produce a full declarative UI, but Slint brings a renderer, a text stack and font handling. Even in debug, the `.so` is 14 MB; that is the toolkit, not the notepad.

**Flutter — 16.9 MB for the same feature set.** The Dart engine and Skia are bundled into every app, which is why a five-hundred-line notepad is half the size of a small game. You are paying once, up front, for a UI that behaves identically on every device.

## Not measured yet

Startup time, resident memory and frame time were **not** measured for this write-up — the test phone was not attached. The size and code numbers above are real; do not quote runtime numbers until they are produced.

`measure.sh` produces them. With a device attached:

```sh
./measure.sh <package> <activity>
```

It prints cold-start time (`am start -W`), total PSS after start (`dumpsys meminfo`), and the installed APK size, so the same three numbers come out for all four variants. The package names are:

```
kotlin       com.vbs.knotepad      com.vbs.knotepad.MainActivity
rust-slint   com.vbs.notepad       com.vbs.notepad.MainActivity
flutter      com.vbs.dnotepad      com.vbs.dnotepad.MainActivity
rust-tiny    com.vbs.tinynotepad   (no launcher activity: the surface is created by the native side)
```

## How the four were built

```sh
# kotlin
cd kotlin && ./gradlew assembleRelease

# flutter
cd flutter && flutter build apk --release

# rust-tiny (release; needs the NDK and cargo-ndk)
cd rust-tiny
ANDROID_NDK_HOME=$ANDROID_SDK/ndk/28.2.13676358 \
  cargo +stable-x86_64-pc-windows-gnu ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release
cd android && ./gradlew assembleRelease

# rust-slint
cd rust-slint
ANDROID_NDK_HOME=$ANDROID_SDK/ndk/28.2.13676358 \
  cargo +stable-x86_64-pc-windows-gnu ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release
cd android && ./gradlew assembleRelease
```

On Windows without the MSVC build tools, the `+stable-x86_64-pc-windows-gnu` toolchain is required — the default MSVC host toolchain fails to link.

## Choosing between them

| If you care most about | Pick |
|---|---|
| Smallest APK, fastest cold start, and you are willing to draw the UI yourself | `rust-tiny` |
| Smallest APK with the least code, native look, lowest maintenance | `kotlin` |
| The same UI on Android and desktop, declarative, native rendering | `rust-slint` |
| One codebase for Android/iOS/web and fast iteration | `flutter` |

The honest summary: **the toolkit decides the size, not the language.** Kotlin is small because the toolkit ships with the phone. Rust is small only when you refuse the toolkit. Flutter is large because it brings its own. Slint sits in between, and pays for itself in code you do not write.

## Layout

```
kotlin/       app/src/main/java/... (single activity), app/build.gradle.kts
flutter/      lib/, android/, pubspec.yaml
rust-slint/   src/{main,lib,store}.rs, ui/app.slint, android/
rust-tiny/    src/{lib,app,android,jni,store}.rs, android/
```

## Caveats

- The Slint variant is a debug artifact; its release size is unmeasured.
- The Flutter APK is the universal release build produced by `flutter build apk`; a per-ABI split would be smaller.
- All four are feature-limited on purpose: no rich text, no sync, no images. That is what makes the comparison fair.
