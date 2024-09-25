Normally windows hide all windows on all monitors when WIN+D is pressed. This tray application instead hides only windows on the monitor with the active focused window.

Specifically, it uses a low-level keyboard hook to capture the keypress and prevent the default behavior of WIN+D. When the keypress is triggered it enumerates all windows and filters away those that are special, hidden, or not on the monitor with the currently focused window. To terminate the application it also adds a tray icon that can be right-clicked and to exit the program.

**Unlike** the default windows implementation which unhides the same windows just hidden (unless any change has been made to them) this does not unhide windows.
