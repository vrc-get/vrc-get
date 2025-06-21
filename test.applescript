#!/usr/bin/env osascript

on run (folderName)
    tell application "Finder"
        tell folder folderName
            open
            
            set theXOrigin to 10
            set theYOrigin to 60
            set theWidth to 500
            set theHeight to 350
            
            set theBottomRightX to (theXOrigin + theWidth)
            set theBottomRightY to (theYOrigin + theHeight)
            -- set dsStore to "\"" & "/Volumes/" & volumeName & "/" & ".DS_STORE\""

            
            tell container window
                set current view to icon view
                set toolbar visible to false
                set statusbar visible to false
                set the bounds to {theXOrigin, theYOrigin, theBottomRightX, theBottomRightY}
                set statusbar visible to false
                -- REPOSITION_HIDDEN_FILES_CLAUSE
            end tell

            
            set opts to the icon view options of container window
            tell opts
                -- set icon size to ICON_SIZE
                -- set text size to TEXT_SIZE
                set arrangement to not arranged
            end tell
            -- BACKGROUND_CLAUSE

        end tell
    end tell
end run
