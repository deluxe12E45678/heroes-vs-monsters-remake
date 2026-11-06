#!/bin/bash
# Android build: cargo-apk (arm64 .so + assets + manifest), then add the Java activity (classes.dex), align and re-sign.
set -e
cd "$(dirname "$0")"
# Prefer restored toolchain under $HOME/android, else legacy /opt/android
if [ -f "$HOME/android/env.sh" ]; then source "$HOME/android/env.sh"
elif [ -f /opt/android/env.sh ]; then source /opt/android/env.sh
fi
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$HOME/android/ndk/27.2.12479018}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/android}"
BT="${ANDROID_SDK_ROOT}/build-tools/35.0.0"
cargo apk build --release --lib --target ${1:-aarch64-linux-android}
rm -rf target/java && mkdir -p target/java/cls target/java/dex
javac --release 8 -nowarn -cp "${ANDROID_SDK_ROOT}/platforms/android-35/android.jar" -d target/java/cls $(find android_java -name '*.java')
$BT/d8 --release --min-api 26 --lib "${ANDROID_SDK_ROOT}/platforms/android-35/android.jar" --output target/java/dex $(find target/java/cls -name '*.class')
cp target/release/apk/hvsm.apk target/java/u.apk
python3 -c "import zipfile; z=zipfile.ZipFile('target/java/u.apk','a'); z.write('target/java/dex/classes.dex','classes.dex'); z.close()"
$BT/zipalign -f -p 4 target/java/u.apk target/java/a.apk
$BT/apksigner sign --ks android_keystore/hvsm.jks --ks-pass pass:hvsmhvsm --out target/hvsm.apk target/java/a.apk
$BT/apksigner verify target/hvsm.apk && ls -la target/hvsm.apk
