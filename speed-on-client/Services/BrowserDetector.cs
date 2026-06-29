using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace SpeedOnClient.Services;

/// <summary>
/// Detects installed web browsers on the current platform and identifies
/// the system default browser.
/// </summary>
public sealed class BrowserDetector
{
    public sealed record BrowserInfo(string Name, string ExecutablePath, bool IsDefault)
    {
        public override string ToString() => Name + (IsDefault ? " (默认)" : "");
    }

    public List<BrowserInfo> DetectBrowsers()
    {
        var browsers = new List<BrowserInfo>();
        var defaultBrowser = DetectDefaultBrowser();

        if (OperatingSystem.IsWindows())
        {
            DetectWindowsBrowsers(browsers, defaultBrowser);
        }
        else if (OperatingSystem.IsMacOS())
        {
            DetectMacOsBrowsers(browsers, defaultBrowser);
        }
        else if (OperatingSystem.IsLinux())
        {
            DetectLinuxBrowsers(browsers, defaultBrowser);
        }

        // Ensure the default browser is first.
        browsers.Sort((a, b) => b.IsDefault.CompareTo(a.IsDefault));
        return browsers;
    }

    // --- Windows ---

    private static void DetectWindowsBrowsers(List<BrowserInfo> browsers, string? defaultBrowser)
    {
        var programDirs = new[]
        {
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData)),
        };

        var knownBrowsers = new (string Name, string[] RelativePaths)[]
        {
            ("Google Chrome", new[] { @"Google\Chrome\Application\chrome.exe" }),
            ("Microsoft Edge", new[] { @"Microsoft\Edge\Application\msedge.exe" }),
            ("Mozilla Firefox", new[] { @"Mozilla Firefox\firefox.exe" }),
            ("Brave", new[] { @"BraveSoftware\Brave-Browser\Application\brave.exe" }),
            ("Opera", new[] { @"Opera\opera.exe" }),
        };

        foreach (var (name, relPaths) in knownBrowsers)
        {
            foreach (var dir in programDirs)
            {
                foreach (var rel in relPaths)
                {
                    var path = Path.Combine(dir, rel);
                    if (File.Exists(path))
                    {
                        var isDefault = string.Equals(name, defaultBrowser, StringComparison.OrdinalIgnoreCase);
                        browsers.Add(new BrowserInfo(name, path, isDefault));
                        break;
                    }
                }
                if (browsers.Exists(b => b.Name == name)) break;
            }
        }
    }

    private static string? DetectDefaultBrowser()
    {
        if (!OperatingSystem.IsWindows()) return null;
        try
        {
            // Query the registry for the default browser ProgID.
            using var key = Microsoft.Win32.Registry.CurrentUser
                .OpenSubKey(@"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice");
            var progId = key?.GetValue("ProgId") as string;
            if (string.IsNullOrEmpty(progId)) return null;

            // Map common ProgIDs to browser names.
            return progId.ToUpperInvariant() switch
            {
                "ChromeHTML" => "Google Chrome",
                "MSEdgeHTM" => "Microsoft Edge",
                "FirefoxURL-308046B0AF4A39CB" => "Mozilla Firefox",
                "BraveHTML" => "Brave",
                "OperaStable" => "Opera",
                _ => null,
            };
        }
        catch
        {
            return null;
        }
    }

    // --- macOS ---

    private static void DetectMacOsBrowsers(List<BrowserInfo> browsers, string? defaultBrowser)
    {
        var apps = new[]
        {
            ("Safari", "/Applications/Safari.app"),
            ("Google Chrome", "/Applications/Google Chrome.app"),
            ("Microsoft Edge", "/Applications/Microsoft Edge.app"),
            ("Mozilla Firefox", "/Applications/Firefox.app"),
            ("Brave", "/Applications/Brave Browser.app"),
            ("Opera", "/Applications/Opera.app"),
        };

        foreach (var (name, path) in apps)
        {
            if (Directory.Exists(path))
            {
                var isDefault = string.Equals(name, defaultBrowser, StringComparison.OrdinalIgnoreCase)
                                || (defaultBrowser == null && name == "Safari");
                browsers.Add(new BrowserInfo(name, path, isDefault));
            }
        }
    }

    // --- Linux ---

    private static void DetectLinuxBrowsers(List<BrowserInfo> browsers, string? defaultBrowser)
    {
        var known = new[]
        {
            ("Google Chrome", new[] { "/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/usr/bin/chromium" }),
            ("Mozilla Firefox", new[] { "/usr/bin/firefox" }),
            ("Microsoft Edge", new[] { "/usr/bin/microsoft-edge" }),
            ("Brave", new[] { "/usr/bin/brave-browser" }),
            ("Opera", new[] { "/usr/bin/opera" }),
        };

        foreach (var (name, paths) in known)
        {
            foreach (var path in paths)
            {
                if (File.Exists(path))
                {
                    browsers.Add(new BrowserInfo(name, path, false));
                    break;
                }
            }
        }
    }
}
