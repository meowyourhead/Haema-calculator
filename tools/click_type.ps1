param(
  [int]$X,
  [int]$Y,
  [string]$Text = "",
  [switch]$Clear
)
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Input {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
"@
[Win32Input]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 120
[Win32Input]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
[Win32Input]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 250
if ($Clear) {
  [System.Windows.Forms.SendKeys]::SendWait("^a")
  Start-Sleep -Milliseconds 80
}
if ($Text -ne "") {
  [System.Windows.Forms.SendKeys]::SendWait($Text)
}
Start-Sleep -Milliseconds 200
