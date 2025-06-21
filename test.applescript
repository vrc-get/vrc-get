#!/usr/bin/env osascript

on run (folderName)
    tell application "Finder"
        tell folder folderName
            log "opening folder " & folderName
            open
            
            set theXOrigin to 10
            set theYOrigin to 60
            set theWidth to 500
            set theHeight to 350
            
            set theBottomRightX to (theXOrigin + theWidth)
            set theBottomRightY to (theYOrigin + theHeight)
            -- set dsStore to "\"" & "/Volumes/" & volumeName & "/" & ".DS_STORE\""
            set dsStore to "\"" & folderName & "/" & ".DS_STORE\""

            log "setting bounds to " & theXOrigin & ", " & theYOrigin & ", " & theBottomRightX & ", " & theBottomRightY
            tell container window
                set current view to icon view
                set toolbar visible to false
                set statusbar visible to false
                set the bounds to {theXOrigin, theYOrigin, theBottomRightX, theBottomRightY}
                set statusbar visible to false
                -- REPOSITION_HIDDEN_FILES_CLAUSE
            end tell

            log "setting window mode to icon view"
            set opts to the icon view options of container window
            tell opts
                -- set icon size to ICON_SIZE
                -- set text size to TEXT_SIZE
                set arrangement to not arranged
            end tell
            -- BACKGROUND_CLAUSE

            -- Positioning
            -- POSITION_CLAUSE

            -- Hiding
            -- HIDING_CLAUSE

            -- Application and QL Link Clauses
            -- APPLICATION_CLAUSE
            -- QL_CLAUSE
            log "reopening folder " & folderName
            close
            open
            -- Force saving of the size
            delay 1

            log "setting status bar invisible"  
            tell container window
                set statusbar visible to false
                set the bounds to {theXOrigin, theYOrigin, theBottomRightX - 10, theBottomRightY - 10}
            end tell
        end tell

        --give the finder some time to write the .DS_Store file
        delay 3

        set waitTime to 0
        set ejectMe to false
        repeat while ejectMe is false
            delay 1
            set waitTime to waitTime + 1
            log "waiting " & waitTime & " seconds for .DS_STORE to be created."
            
            if (do shell script "[ -f " & dsStore & " ]; echo $?") = "0" then set ejectMe to true
        end repeat
        log "waited " & waitTime & " seconds for .DS_STORE to be created."
    end tell
end run
