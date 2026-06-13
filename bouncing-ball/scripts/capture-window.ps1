param(
    [string]$Title = "Bouncing Ball Physics Lab",
    [string]$Out = (Join-Path $env:TEMP "bouncing-ball-window.png"),
    [int]$X = 40,
    [int]$Y = 40,
    [int]$Width = 1120,
    [int]$Height = 720,
    [switch]$NoMove
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Win32Capture {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("dwmapi.dll")]
    public static extern int DwmGetWindowAttribute(IntPtr hwnd, int dwAttribute, out RECT pvAttribute, int cbAttribute);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr hWnd,
        IntPtr hWndInsertAfter,
        int X,
        int Y,
        int cx,
        int cy,
        uint uFlags
    );

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }
}
"@

$matches = New-Object System.Collections.Generic.List[System.IntPtr]

[Win32Capture]::EnumWindows({
    param([IntPtr]$hwnd, [IntPtr]$lparam)

    if (-not [Win32Capture]::IsWindowVisible($hwnd)) {
        return $true
    }

    $text = New-Object System.Text.StringBuilder 512
    [void][Win32Capture]::GetWindowText($hwnd, $text, $text.Capacity)
    if ($text.ToString().Contains($Title)) {
        $matches.Add($hwnd)
    }

    return $true
}, [IntPtr]::Zero) | Out-Null

if ($matches.Count -eq 0) {
    throw "No visible window found containing title '$Title'."
}

$hwnd = $matches[0]

if (-not $NoMove) {
    $SWP_NOZORDER = 0x0004
    $SWP_SHOWWINDOW = 0x0040
    [void][Win32Capture]::SetWindowPos($hwnd, [IntPtr]::Zero, $X, $Y, $Width, $Height, $SWP_NOZORDER -bor $SWP_SHOWWINDOW)
    Start-Sleep -Milliseconds 350
}

[void][Win32Capture]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 250

$rect = New-Object Win32Capture+RECT
$DWMWA_EXTENDED_FRAME_BOUNDS = 9
$dwmResult = [Win32Capture]::DwmGetWindowAttribute(
    $hwnd,
    $DWMWA_EXTENDED_FRAME_BOUNDS,
    [ref]$rect,
    [Runtime.InteropServices.Marshal]::SizeOf([type][Win32Capture+RECT])
)

if ($dwmResult -ne 0 -or $rect.Right -le $rect.Left -or $rect.Bottom -le $rect.Top) {
    [void][Win32Capture]::GetWindowRect($hwnd, [ref]$rect)
}

$captureWidth = [Math]::Max(1, $rect.Right - $rect.Left)
$captureHeight = [Math]::Max(1, $rect.Bottom - $rect.Top)

$outDir = Split-Path -Parent $Out
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$bitmap = New-Object System.Drawing.Bitmap $captureWidth, $captureHeight
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, [System.Drawing.Size]::new($captureWidth, $captureHeight))
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

Write-Output $Out
