#!/usr/bin/env osascript

on run (folderName)
    tell application "Finder"
        activate
        open folder folderName
    end tell
end run
