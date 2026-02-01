#!/bin/sh
set -u
set -e

#HOST=raspberrynut.local
#REMOTE=/home/nathan/Documents/pico
#DEVICE=/dev/ttyACM0
#PICOTOOL=/usr/bin/picotool
#echo rsyncing...
#rsync $1 $HOST:$REMOTE
#echo uploading...
#ssh $HOST $PICOTOOL load -f -u -v -x -t elf $REMOTE
#ssh $HOST "bash -c 'while [ ! -e $DEVICE ]; do sleep 1; done'"
#reset
#ssh -t $HOST /usr/bin/picocom $DEVICE
#reset

HOST=nathans-macbook-pro-2.local
REMOTE=/Users/nathan/Documents/firmware
#DEVICE=/dev/cu.usbmodem14101
DEVICE=/dev/cu.usbmodem14201
PICOTOOL=/usr/local/bin/picotool
echo rsyncing...
rsync $1 $HOST:$REMOTE
echo uploading...
ssh $HOST $PICOTOOL load -f -u -v -x -t elf $REMOTE
ssh $HOST "bash -c 'while [ ! -e $DEVICE ]; do sleep 1; done'"
reset
ssh -t $HOST /usr/local/bin/tio -n $DEVICE
reset

#DEVICE=/dev/cu.usbmodem101
#PICOTOOL=/opt/homebrew/bin/picotool
#TIO=/opt/homebrew/bin/tio
#echo uploading...
#$PICOTOOL load -f -u -v -x -t elf $1
#while [ ! -e $DEVICE ]; do sleep 1; done
#reset
#$TIO -n $DEVICE
#reset