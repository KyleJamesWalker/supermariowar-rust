#!/bin/bash
# HELP: Super Mario War - stomp your friends in 4-player platform battles
# ICON: smw

. /opt/muos/script/var/func.sh

if command -v SETUP_APP >/dev/null 2>&1; then
	SETUP_APP smw ""
else
	echo app >/tmp/act_go
	SETUP_SDL_ENVIRONMENT
fi

# Derived from $0, not GET_VAR: the app may be installed on either card.
APP_DIR="$(cd "$(dirname "$0")" && pwd)"
export HOME="$APP_DIR/home"
export LD_LIBRARY_PATH="$APP_DIR/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export SMW_NO_KEYBOARD=1 SMW_PAD_TRANSLATE=1
# export SMW_DEBUG_INPUT=1
# A comma-separated list SDL tries in order; "dummy" runs silent.
AUDIO_DRIVER="alsa"
export SDL_AUDIODRIVER="$AUDIO_DRIVER"
mkdir -p "$HOME"
cd "$APP_DIR" || exit 1
chmod +x ./smw 2>/dev/null

# The event nodes the kernel also gives a jsN handler; only muOS's virtual pad when it exists.
if [ -z "$SDL_JOYSTICK_DEVICE" ]; then
	PADS="$(awk '/^N: / { name = $0 } /^H: / && (/js[0-9]/ || name ~ /"muOS-Keys"/) && match($0, /event[0-9]+/) { node = "/dev/input/" substr($0, RSTART, RLENGTH); if (name ~ /"muOS-Keys"/) first = first (first ? ":" : "") node; else rest = rest (rest ? ":" : "") node } END { print first ? first : rest }' /proc/bus/input/devices)"
	[ -n "$PADS" ] && export SDL_JOYSTICK_DEVICE="$PADS"
fi

{
	echo "=== muOS $(cat /opt/muos/config/system/version /opt/muos/config/version.txt 2>/dev/null | head -n 1)"
	echo "=== /proc/bus/input/devices"
	cat /proc/bus/input/devices
	echo "=== /dev/input"
	ls -l /dev/input /dev/input/by-id 2>&1
	echo "=== environment"
	env | grep -E '^(SDL_|HOME=|LD_)' | grep -v '^SDL_GAMECONTROLLERCONFIG='
	echo "=== udev"
	pgrep -a udevd || echo "udevd not running"
	echo "=== game"
} >"$APP_DIR/log.txt" 2>&1

./smw --datadir "$APP_DIR/data" >>"$APP_DIR/log.txt" 2>&1

SCREEN_TYPE="internal"
[ "$(GET_VAR config boot/device_mode)" = "1" ] && SCREEN_TYPE="external"
FB_SWITCH "$(GET_VAR device screen/${SCREEN_TYPE}/width)" "$(GET_VAR device screen/${SCREEN_TYPE}/height)" 32

sync
