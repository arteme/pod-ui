#!/bin/bash
convert icon.png -resize 256x256 icon-256.png
convert icon-256.png -resize 16x16 icon-16.png
convert icon-256.png -resize 32x32 icon-32.png
convert icon-256.png -resize 64x64 icon-64.png
convert icon-256.png -resize 128x128 icon-128.png
convert icon-16.png icon-32.png icon-64.png icon-128.png icon-256.png -colors 256 icon.ico


