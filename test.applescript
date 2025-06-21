#!/usr/bin/env osascript

on run (folderName)
    tell application "Finder"
        tell folder folderName
            log "opening folder " & folderName
            open
        end tell
    end tell
end run
