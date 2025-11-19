#!/bin/sh
set -u
set -e
REMOTE=/home/nathan/Documents/pico
echo rsyncing...
rsync $1 raspberrynut.local:$REMOTE
echo uploading...
ssh raspberrynut.local picotool load -u -v -x -t elf $REMOTE
ssh raspberrynut.local "bash -c 'while [ ! -e /dev/ttyACM0 ]; do sleep 1; done'"
ssh -t raspberrynut.local screen /dev/ttyACM0
