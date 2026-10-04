#!/bin/bash
# HELP: Super Mario War - stomp your friends in 4-player platform battles
# ICON: smw

. /opt/muos/script/var/func.sh
echo app >/tmp/act_go

SETUP_SDL_ENVIRONMENT

# Derived from $0, not GET_VAR: the app may be installed on either card.
APP_DIR="$(cd "$(dirname "$0")" && pwd)"
export HOME="$APP_DIR/home"
export LD_LIBRARY_PATH="$APP_DIR/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
mkdir -p "$HOME"
cd "$APP_DIR" || exit 1
chmod +x ./smw 2>/dev/null

./smw --datadir "$APP_DIR/data" >"$APP_DIR/log.txt" 2>&1

SCREEN_TYPE="internal"
[ "$(GET_VAR config boot/device_mode)" = "1" ] && SCREEN_TYPE="external"
FB_SWITCH "$(GET_VAR device screen/${SCREEN_TYPE}/width)" "$(GET_VAR device screen/${SCREEN_TYPE}/height)" 32

sync
