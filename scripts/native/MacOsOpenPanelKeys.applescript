property pollDelay : 0.2
property pollAttempts : 75
property returnKey : 36
property escapeKey : 53
property closePolls : 10
property closePresses : 7
property pidFileName : "picker.pid"

on run arguments
    set appPid to my appPid(item 1 of arguments)
    set chosenPath to item 2 of arguments
    tell application "System Events"
        if not UI elements enabled then error "Accessibility access is required to answer the open panel"
    end tell
    my waitUntil(my PanelIsOpen, appPid, "", "the app to open its open panel")
    if chosenPath is "" then
        my pressUntilClosed(appPid, escapeKey, "Escape")
    else
        my goToFile(appPid, chosenPath)
        my pressUntilClosed(appPid, returnKey, "Return")
    end if
    return "answered the open panel of PID " & appPid
end run

on appPid(target)
    if target does not start with "/" then return target as integer
    return my waitForPidFile(target & "/" & pidFileName)
end appPid

on waitForPidFile(pidPath)
    repeat pollAttempts times
        try
            return (read POSIX file pidPath) as integer
        end try
        delay pollDelay
    end repeat
    error "Waited " & (pollAttempts * pollDelay) & " seconds for the app to write its PID to " & pidPath
end waitForPidFile

on waitFor(condition, appPid, expected, attempts)
    repeat attempts times
        if condition's isMet(appPid, expected) then return true
        delay pollDelay
    end repeat
    return false
end waitFor

on waitUntil(condition, appPid, expected, waitedFor)
    if my waitFor(condition, appPid, expected, pollAttempts) then return
    error "Waited " & (pollAttempts * pollDelay) & " seconds for " & waitedFor & " (PID " & appPid & ", windows:" & my windowSummary(appPid) & ")"
end waitUntil

on bringToFront(appPid)
    tell application "System Events" to set frontmost of (first process whose unix id is appPid) to true
    my waitUntil(my AppIsFrontmost, appPid, "", "the app to come to the front")
end bringToFront

on goToFile(appPid, chosenPath)
    my bringToFront(appPid)
    tell application "System Events" to keystroke "g" using {command down, shift down}
    my waitUntil(my SheetIsOpen, appPid, "", "the Go to Folder sheet to open")
    my bringToFront(appPid)
    tell application "System Events"
        keystroke "a" using {command down}
        keystroke chosenPath
    end tell
    my waitUntil(my FieldHoldsPath, appPid, chosenPath, "the Go to Folder field to hold " & chosenPath)
    my bringToFront(appPid)
    tell application "System Events" to key code returnKey
    my waitUntil(my SheetIsClosed, appPid, "", "the Go to Folder sheet to close")
end goToFile

on pressUntilClosed(appPid, keyCode, keyName)
    repeat closePresses times
        if my PanelIsClosed's isMet(appPid, "") then return
        my pressKey(appPid, keyCode)
        if my waitFor(my PanelIsClosed, appPid, "", closePolls) then return
    end repeat
    error "Pressed " & keyName & " " & closePresses & " times but the open panel stayed open (PID " & appPid & ", windows:" & my windowSummary(appPid) & ")"
end pressUntilClosed

on pressKey(appPid, keyCode)
    my bringToFront(appPid)
    tell application "System Events" to key code keyCode
end pressKey

on windowSummary(appPid)
    set summary to ""
    try
        tell application "System Events"
            tell first process whose unix id is appPid
                repeat with index from 1 to count of windows
                    set summary to summary & " " & (name of window index) & " (" & (subrole of window index) & ")"
                end repeat
            end tell
        end tell
    on error message
        set summary to summary & " unreadable: " & message
    end try
    return summary
end windowSummary

script PanelIsOpen
    on isMet(appPid, expected)
        try
            tell application "System Events"
                tell first process whose unix id is appPid
                    return exists (first window whose subrole is "AXDialog")
                end tell
            end tell
        end try
        return false
    end isMet
end script

script PanelIsClosed
    on isMet(appPid, expected)
        try
            tell application "System Events"
                tell first process whose unix id is appPid
                    return not (exists (first window whose subrole is "AXDialog"))
                end tell
            end tell
        end try
        return false
    end isMet
end script

script AppIsFrontmost
    on isMet(appPid, expected)
        try
            tell application "System Events"
                return frontmost of (first process whose unix id is appPid)
            end tell
        end try
        return false
    end isMet
end script

script SheetIsOpen
    on isMet(appPid, expected)
        try
            tell application "System Events"
                tell first process whose unix id is appPid
                    return exists text field 1 of sheet 1 of (first window whose subrole is "AXDialog")
                end tell
            end tell
        end try
        return false
    end isMet
end script

script SheetIsClosed
    on isMet(appPid, expected)
        try
            tell application "System Events"
                tell first process whose unix id is appPid
                    if not (exists (first window whose subrole is "AXDialog")) then return true
                    return not (exists sheet 1 of (first window whose subrole is "AXDialog"))
                end tell
            end tell
        end try
        return false
    end isMet
end script

script FieldHoldsPath
    on isMet(appPid, expected)
        try
            tell application "System Events"
                tell first process whose unix id is appPid
                    return (value of text field 1 of sheet 1 of (first window whose subrole is "AXDialog")) is expected
                end tell
            end tell
        end try
        return false
    end isMet
end script
