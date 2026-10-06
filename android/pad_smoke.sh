#!/usr/bin/env bash
# Plugs a virtual Bluetooth Xbox pad (android/vpad.c, built for the device's ABI) into the connected device or
# emulator, launches the installed debug APK with it connected, presses the D-pad, then unplugs and replugs it.
# Fails when the game exits or a step's [pad]/[input] line does not reach logcat and log.txt. Needs a rootable image.
# Usage: android/pad_smoke.sh <vpad binary>
set -uo pipefail

vpad=$1
pkg=com.kylejameswalker.supermariowar
input=/data/local/tmp/smw-pad
out="${SMOKE_OUT:-${TMPDIR:-/tmp}/smw-android-smoke}/pad"
mkdir -p "$out"

plug() {
    adb shell ": > $input; (nohup sh -c 'tail -f $input | /data/local/tmp/vpad \"Xbox Wireless Controller\" 0x045e 0x02e0 5' > $input.log 2>&1 &)"
}
send() {
    for line in "$@"; do adb shell "echo '$line' >> $input"; done
}
# wait_for <count> <pattern>: until logcat has <count> lines matching <pattern>, for up to 20 s.
wait_for() {
    for _ in $(seq 40); do
        [[ "$(adb logcat -d -s smw | grep -cE "$2")" -ge $1 ]] && return 0
        sleep 0.5
    done
    return 1
}
fail() {
    echo "pad smoke: $1"
    adb logcat -d > "$out/logcat.txt"
    grep -E ' (smw|SDL|AndroidRuntime|DEBUG|libc) ' "$out/logcat.txt" | grep -v 'I smw *: loading' | tail -n 40
    finish
    exit 1
}
finish() {
    adb shell am force-stop "$pkg"
    adb shell "pkill -f 'tail -f $input'; pkill -f /data/local/tmp/vpad" 2> /dev/null
}

# Only root reads another app's Android/data on Android 11 and later.
adb root > /dev/null && adb wait-for-device
adb push "$vpad" /data/local/tmp/vpad > /dev/null && adb shell chmod 755 /data/local/tmp/vpad || exit 2
adb shell pm clear "$pkg" > /dev/null
plug
sleep 2
adb shell cat "$input.log" | grep -q created || fail "cannot create the virtual pad: $(adb shell cat "$input.log")"
adb logcat -c
adb shell am start -W -n "$pkg/.MainActivity" --es SMW_DEBUG_INPUT 1 > /dev/null

wait_for 1 '\[pad\] joystick 0: .*\(translated\)' || fail "the pad at launch was not translated"
wait_for 1 '\[loop\] frame 240' || fail "the game did not reach frame 240"
send "a 17 1" "s 100" "a 17 0"
wait_for 1 'game jhat which 0 hat 0 value 4' || fail "D-pad down did not reach the game as hat 0"

send q
wait_for 1 '\[pad\] joystick 0 removed' || fail "unplugging was not noticed"
plug
wait_for 2 '\[pad\] joystick 0: .*\(translated\)' || fail "the replugged pad was not opened as joystick 0"
send "k 304 1" "s 100" "k 304 0"
wait_for 1 'game jbutton which 0 button 0 state 1' || fail "A on the replugged pad did not reach the game as button 0"

[[ -n "$(adb shell pidof "$pkg" | tr -d '\r')" ]] || fail "the game exited"
adb exec-out cat "/sdcard/Android/data/$pkg/files/log.txt" > "$out/log.txt"
grep -q '\[pad\] joystick 0 removed' "$out/log.txt" || fail "log.txt lacks the [pad] lines"
finish
echo "pad smoke: OK"
