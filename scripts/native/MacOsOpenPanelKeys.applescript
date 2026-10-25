on run arguments
    set appPid to (item 1 of arguments) as integer
    set chosenPath to item 2 of arguments
    tell application "System Events"
        if not UI elements enabled then error "Accessibility access is required to answer the open panel"
        tell first process whose unix id is appPid
            set frontmost to true
            my waitForPanel(appPid)
            if chosenPath is "" then
                key code 53
            else
                keystroke "g" using {command down, shift down}
                delay 1
                keystroke chosenPath
                key code 36
                delay 1
                key code 36
            end if
        end tell
    end tell
    return "answered the open panel of PID " & appPid
end run

on waitForPanel(appPid)
    repeat 100 times
        tell application "System Events"
            tell first process whose unix id is appPid
                if (count of windows) > 1 then return
            end tell
        end tell
        delay 0.1
    end repeat
    error "The app opened no panel within ten seconds"
end waitForPanel
