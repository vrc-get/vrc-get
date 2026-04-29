#!/bin/sh -eu

cd "$(dirname "$0")" || exit 1

cargo update
cd vrc-get-gui
npm update
