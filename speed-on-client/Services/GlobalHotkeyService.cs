using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace SpeedOnClient.Services;

/// <summary>
/// Registers a global hotkey (Win+Alt) to toggle the search window.
/// On Windows uses a low-level keyboard hook so the combination
/// Win+Alt (without any additional key) can be captured.
/// On macOS a CGEventTap stub is provided; full implementation requires
/// accessibility permissions.
/// </summary>
public sealed class GlobalHotkeyService
{
    public event Action? HotkeyPressed;

    private bool _registered;
    private bool _winDown;
    private bool _altDown;
    private bool _triggered;

    // --- Windows P/Invoke ---

    private delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);
    private LowLevelKeyboardProc? _proc;
    private IntPtr _hookId = IntPtr.Zero;

    private const int WH_KEYBOARD_LL = 13;
    private const int WM_KEYDOWN = 0x0100;
    private const int WM_KEYUP = 0x0101;
    private const int WM_SYSKEYDOWN = 0x0104;
    private const int WM_SYSKEYUP = 0x0105;

    private const int VK_LWIN = 0x5B;
    private const int VK_RWIN = 0x5C;
    private const int VK_LMENU = 0xA4; // Left Alt
    private const int VK_RMENU = 0xA5; // Right Alt

    [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

    [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UnhookWindowsHookEx(IntPtr hhk);

    [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);

    [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr GetModuleHandle(string lpModuleName);

    public void Register()
    {
        if (_registered) return;

        if (OperatingSystem.IsWindows())
        {
            RegisterWindows();
        }
        else if (OperatingSystem.IsMacOS())
        {
            // macOS: CGEventTap requires accessibility permissions.
            // A full implementation would P/Invoke CoreGraphics.
            // For now, we leave a placeholder — the app can be activated
            // by clicking the tray icon on macOS.
            Debug.WriteLine("macOS global hotkey not yet implemented — use tray icon.");
        }

        _registered = true;
    }

    private void RegisterWindows()
    {
        _proc = HookCallback;
        using var curProcess = System.Diagnostics.Process.GetCurrentProcess();
        using var curModule = curProcess.MainModule!;
        _hookId = SetWindowsHookEx(WH_KEYBOARD_LL, _proc, GetModuleHandle(curModule.ModuleName!), 0);
    }

    private IntPtr HookCallback(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode >= 0)
        {
            int vkCode = Marshal.ReadInt32(lParam);
            int msg = wParam.ToInt32();

            bool isDown = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            bool isUp = msg == WM_KEYUP || msg == WM_SYSKEYUP;

            if (vkCode == VK_LWIN || vkCode == VK_RWIN)
            {
                _winDown = isDown;
                if (isUp) _triggered = false;
            }

            if (vkCode == VK_LMENU || vkCode == VK_RMENU)
            {
                _altDown = isDown;
                if (isUp) _triggered = false;
            }

            // Trigger when both Win and Alt are held and we haven't fired yet.
            // Only trigger on the modifier key-down event itself (not regular keys).
            if (isDown && _winDown && _altDown && !_triggered && IsModifierKey(vkCode))
            {
                _triggered = true;
                HotkeyPressed?.Invoke();
            }
        }

        return CallNextHookEx(_hookId, nCode, wParam, lParam);
    }

    private static bool IsModifierKey(int vk) =>
        vk == VK_LWIN || vk == VK_RWIN || vk == VK_LMENU || vk == VK_RMENU;

    public void Unregister()
    {
        if (!_registered) return;

        if (OperatingSystem.IsWindows() && _hookId != IntPtr.Zero)
        {
            UnhookWindowsHookEx(_hookId);
            _hookId = IntPtr.Zero;
        }

        _registered = false;
        _winDown = false;
        _altDown = false;
        _triggered = false;
    }
}
