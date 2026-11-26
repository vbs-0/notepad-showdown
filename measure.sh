#!/bin/sh
# Measure cold start and memory for one installed variant.
#   ./measure.sh com.vbs.knotepad com.vbs.knotepad.MainActivity
# Needs a device attached (adb devices must list it).
set -e
pkg="$1"
activity="$2"
[ -n "$pkg" ] || { echo "usage: $0 <package> [activity]"; exit 1; }

echo "== $pkg"
adb shell am force-stop "$pkg" 2>/dev/null || true
if [ -n "$activity" ]; then
  adb shell am start -W -n "$pkg/$activity" | sed -n 's/^TotalTime: /cold start (ms): /p'
else
  adb shell monkey -p "$pkg" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
  echo "cold start: not available without a launcher activity"
fi
sleep 2
echo "total PSS: $(adb shell dumpsys meminfo "$pkg" | sed -n 's/.*TOTAL PSS: *\([0-9]*\).*/\1 KB/p' | head -1)"
echo "apk on device: $(adb shell pm path "$pkg" | sed -n 's/package://p' | head -1)"
