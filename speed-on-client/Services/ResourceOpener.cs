using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace SpeedOnClient.Services;

/// <summary>
/// Opens resources (URLs, search queries, applications) using the
/// appropriate platform mechanism.
/// </summary>
public static class ResourceOpener
{
    /// <summary>
    /// Open a URL in a specific browser executable.
    /// </summary>
    public static void OpenUrlInBrowser(string browserPath, string url)
    {
        try
        {
            if (OperatingSystem.IsMacOS())
            {
                // On macOS, browserPath is an .app bundle directory.
                Process.Start("open", $"-a \"{browserPath}\" \"{url}\"");
            }
            else if (OperatingSystem.IsWindows())
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = browserPath,
                    Arguments = $"\"{url}\"",
                    UseShellExecute = false,
                });
            }
            else
            {
                Process.Start(browserPath, $"\"{url}\"");
            }
        }
        catch { /* best effort */ }
    }

    /// <summary>
    /// Open a URL in the system default browser.
    /// </summary>
    public static void OpenUrlDefault(string url)
    {
        try
        {
            if (OperatingSystem.IsWindows())
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true,
                });
            }
            else if (OperatingSystem.IsMacOS())
            {
                Process.Start("open", url);
            }
            else
            {
                Process.Start("xdg-open", url);
            }
        }
        catch { /* best effort */ }
    }

    /// <summary>
    /// Open a search-engine URL.
    /// </summary>
    public static void OpenSearchEngine(string searchUrl)
    {
        OpenUrlDefault(searchUrl);
    }
}
