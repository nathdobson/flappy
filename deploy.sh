#!/usr/bin/env bash
set -e
set -u
SKETCH=$1
FQBN=arduino:renesas_uno:unor4wifi
PI_CLI=/home/linuxbrew/.linuxbrew/bin/arduino-cli
ARDUINO_TTY=/dev/ttyACM0
echo "Compiling $SKETCH..."
cd $SKETCH
arduino-cli compile --fqbn $FQBN $SKETCH.ino --output-dir target --libraries=libraries
echo Copying...
rsync -r target raspberrynut.local:/home/nathan/workspace/$SKETCH
echo Initiating...
ssh raspberrynut.local -t "
    set -e
    set -u
    cd /home/nathan/workspace/$SKETCH
    echo Uploading...
    $PI_CLI upload --build-path target -p $ARDUINO_TTY -b $FQBN
    echo Monitoring...
    $PI_CLI monitor -p $ARDUINO_TTY -c 115200"
echo done
