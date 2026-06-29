using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Input;
using Avalonia.Markup.Xaml;
using SpeedOnClient.Services;
using SpeedOnClient.Views;

namespace SpeedOnClient;

public partial class App : Application
{
    private TrayIcon? _trayIcon;
    private MainWindow? _mainWindow;
    private GlobalHotkeyService? _hotkeyService;
    private CoreIpcClient? _coreClient;

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            // Prevent the app from exiting when the main window is closed/hidden.
            desktop.ShutdownMode = ShutdownMode.OnExplicitShutdown;

            // Start the Rust core IPC client.
            _coreClient = new CoreIpcClient();
            _coreClient.StartAsync().ContinueWith(t =>
            {
                if (t.IsFaulted)
                    System.Diagnostics.Debug.WriteLine($"Core IPC failed to start: {t.Exception}");
            });

            // Create the main (search) window but keep it hidden initially.
            _mainWindow = new MainWindow(_coreClient);
            desktop.MainWindow = _mainWindow;

            // Set up the global hotkey (Win+Alt).
            _hotkeyService = new GlobalHotkeyService();
            _hotkeyService.HotkeyPressed += () =>
            {
                Avalonia.Threading.Dispatcher.UIThread.Post(() => ToggleMainWindow());
            };
            _hotkeyService.Register();

            // Create the system tray icon.
            SetupTrayIcon();
        }

        base.OnFrameworkInitializationCompleted();
    }

    private void ToggleMainWindow()
    {
        if (_mainWindow == null) return;

        if (_mainWindow.IsVisible)
            _mainWindow.HideWindow();
        else
            _mainWindow.ShowWindow();
    }

    private void SetupTrayIcon()
    {
        var showItem = new NativeMenuItem("显示搜索窗口");
        showItem.Click += (_, _) =>
            Avalonia.Threading.Dispatcher.UIThread.Post(() => _mainWindow?.ShowWindow());

        var refreshItem = new NativeMenuItem("刷新应用索引");
        refreshItem.Click += (_, _) => { _ = _coreClient?.RefreshApplicationsAsync(); };

        var exitItem = new NativeMenuItem("退出");
        exitItem.Click += (_, _) =>
        {
            _hotkeyService?.Unregister();
            _coreClient?.Dispose();
            _trayIcon?.Dispose();
            (ApplicationLifetime as IClassicDesktopStyleApplicationLifetime)?.Shutdown(0);
        };

        var menu = new NativeMenu();
        menu.Items.Add(showItem);
        menu.Items.Add(refreshItem);
        menu.Items.Add(new NativeMenuItemSeparator());
        menu.Items.Add(exitItem);

        _trayIcon = new TrayIcon
        {
            ToolTipText = "Speed-On",
            Menu = menu,
            IsVisible = true,
        };
    }
}
