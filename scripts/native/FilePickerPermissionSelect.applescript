on run arguments
    set pidPath to (item 1 of arguments) & "/picker.pid"
    set selectedPath to item 2 of arguments
    set fixturePid to my waitForPid(pidPath)
    tell application "System Events"
        if not UI elements enabled then error "Accessibility access is required for the owned picker fixture"
        tell first process whose unix id is fixturePid
            set frontmost to true
            my waitForPanel(fixturePid)
            keystroke "g" using {command down, shift down}
            delay 1
            keystroke selectedPath
            key code 36
            delay 1
            key code 36
        end tell
    end tell
    return "selected task-local file for verified fixture PID " & fixturePid
end run

on waitForPid(pidPath)
    repeat 100 times
        try
            return (read POSIX file pidPath) as integer
        end try
        delay 0.1
    end repeat
    error "The owned picker did not publish its PID within ten seconds"
end waitForPid

on waitForPanel(fixturePid)
    repeat 100 times
        tell application "System Events"
            tell first process whose unix id is fixturePid
                if exists window 1 then return
            end tell
        end tell
        delay 0.1
    end repeat
    error "The owned picker did not open a window within ten seconds"
end waitForPanel
