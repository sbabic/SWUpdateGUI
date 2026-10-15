#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
#
# Build and install LVGL with the configuration needed by the recovery GUI.
#
#   LVGL_VERSION  git tag to build (default: v9.6.0)
#   PREFIX        installation prefix (default: /usr/local)

set -eu

LVGL_VERSION="${LVGL_VERSION:-v9.6.0}"
PREFIX="${PREFIX:-/usr/local}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

git clone --quiet --depth 1 --branch "$LVGL_VERSION" \
	https://github.com/lvgl/lvgl.git "$WORKDIR/lvgl"

# lv_conf.h is installed together with the LVGL headers
conf="$WORKDIR/lv_conf.h"
sed -e '0,/^#if 0/s//#if 1/' "$WORKDIR/lvgl/lv_conf_template.h" > "$conf"

for option in LV_USE_LINUX_FBDEV LV_USE_EVDEV LV_USE_GRIDNAV LV_USE_KEYBOARD \
	LV_FONT_MONTSERRAT_14; do
	sed -i -e "s/^\(\s*#define $option\)\s.*/\1 1/" "$conf"
	grep -q "^\s*#define $option 1$" "$conf" || {
		echo "Failed to enable $option in lv_conf.h" >&2
		exit 1
	}
done

cmake -S "$WORKDIR/lvgl" -B "$WORKDIR/build" \
	-DLV_BUILD_CONF_PATH="$conf" \
	-DCMAKE_BUILD_TYPE=Release \
	-DCMAKE_INSTALL_PREFIX="$PREFIX" \
	-DCONFIG_LV_BUILD_EXAMPLES=OFF \
	-DCONFIG_LV_BUILD_DEMOS=OFF
cmake --build "$WORKDIR/build" --parallel "$(nproc)"
cmake --install "$WORKDIR/build"
