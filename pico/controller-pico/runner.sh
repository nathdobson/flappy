#!/bin/sh
set -u
set -e
#REMOTE=/home/nathan/Documents/pico
#echo rsyncing...
#rsync $1 raspberrynut.local:$REMOTE
#echo uploading...
#ssh raspberrynut.local picotool load -u -v -x -t elf $REMOTE
#ssh raspberrynut.local "bash -c 'while [ ! -e /dev/ttyACM0 ]; do sleep 1; done'"
#ssh -t raspberrynut.local screen /dev/ttyACM0

#picotool load -u -v -x -t elf $1
#bash -c 'while [ ! -e /dev/cu.usbmodem101 ]; do sleep 1; done'
#reset
#screen /dev/cu.usbmodem101
#reset

HOST=nathans-macbook-pro-2.local
REMOTE=/Users/nathan/Documents/pico
DEVICE=/dev/cu.usbmodem14101
PICOTOOL=/usr/local/bin/picotool
echo rsyncing...
rsync $1 $HOST:$REMOTE
echo terminating...
# Set the baud rate to 50 as an out-of-bounds signal
ssh $HOST stty -f $DEVICE 50 || true
ssh $HOST "bash -c 'while [ -e $DEVICE ]; do sleep 1; done'"
echo uploading...
ssh $HOST $PICOTOOL load -f -u -v -x -t elf $REMOTE
ssh $HOST "bash -c 'while [ ! -e $DEVICE ]; do sleep 1; done'"
ssh -t $HOST screen $DEVICE
