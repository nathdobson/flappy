#!/usr/bin/env bash
set -e
set -u
set -e -o pipefail
SKETCH=$1
FQBN=arduino:renesas_uno:unor4wifi
PI_CLI=/home/linuxbrew/.linuxbrew/bin/arduino-cli
ARDUINO_TTY=ttyACM0
echo "Compiling $SKETCH..."
cd $SKETCH
arduino-cli compile --fqbn $FQBN $SKETCH.ino --output-dir target --libraries=libraries | sed 's/^/    /'
echo Staging...
rsync -r --delete target raspberrynut.local:/home/nathan/workspace/$SKETCH | sed 's/^/    /'
echo Connecting to SSH...
ssh raspberrynut.local -t "
    set -e
    set -u
    cd /home/nathan/workspace/$SKETCH
    echo Uploading...
    $PI_CLI upload --build-path target -p /dev/$ARDUINO_TTY -b $FQBN > /dev/null
    echo Connecting to Serial...
    { echo '^'; cat; } | $PI_CLI monitor -q -p /dev/$ARDUINO_TTY -c 115200"
echo done
