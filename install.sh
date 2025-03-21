#!/bin/bash

# URL of the file to download
FILE_URL="https://portapad.kotoca.net/files/service"

# Destination directory
DEST_DIR="/opt/service"

# Create the destination directory if it doesn't exist
mkdir -p "$DEST_DIR"

# Download the file
curl -o "$DEST_DIR/$(basename $FILE_URL)" "$FILE_URL"

# Check if the download was successful
if [ $? -eq 0 ]; then
    echo "本体のダウンロード完了"
else
    echo "Curl中のエラー"
    exit 1
fi

