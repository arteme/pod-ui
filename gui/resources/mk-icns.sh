#!/bin/bash
I=icon.iconset
mkdir $I
convert icon.png -resize 16x16 $I/icon_16x16.png
convert icon.png -resize 32x32 $I/icon_16x16@2.png
convert icon.png -resize 32x32 $I/icon_32x32.png
convert icon.png -resize 64x64 $I/icon_32x32@2.png
convert icon.png -resize 64x64 $I/icon_64x64.png
convert icon.png -resize 128x128 $I/icon_64x64@2.png
convert icon.png -resize 128x128 $I/icon_128x128.png
convert icon.png -resize 256x256 $I/icon_128x128@2.png
convert icon.png -resize 256x256 $I/icon_256x256.png
convert icon.png -resize 512x512 $I/icon_256x256@2.png
convert icon.png -resize 512x512 $I/icon_512x512.png
#convert icon.png -resize 1024x1024 $I/icon_512x512@2.png
#convert icon.png -resize 1024x1024 $I/icon_1024x1024.png

iconutil -c icns $I
rm -rf $I
