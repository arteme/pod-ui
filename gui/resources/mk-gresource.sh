#!/bin/bash
I=icon
mkdir $I
convert icon.png -resize 16x16 $I/16x16.png
convert icon.png -resize 32x32 $I/32x32.png
convert icon.png -resize 64x64 $I/64x64.png
convert icon.png -resize 128x128 $I/128x128.png
convert icon.png -resize 256x256 $I/256x256.png
convert icon.png -resize 512x512 $I/512x512.png

glib-compile-resources icon.gresource.xml
rm -rf $I
