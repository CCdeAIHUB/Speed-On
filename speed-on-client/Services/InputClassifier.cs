using System;
using System.Text.RegularExpressions;

namespace SpeedOnClient.Services;

/// <summary>
/// Classifies user input into one of three categories that determine
/// which results to display:
/// <list type="bullet">
///   <item><b>Url</b> — the input looks like a web address → show browsers.</item>
///   <item><b>Search</b> — general text → show search engines (and core results if any).</item>
///   <item><b>Empty</b> — no input → show core recommendations.</item>
/// </list>
/// </summary>
public enum InputCategory
{
    Empty,
    Url,
    Search,
}

public static class InputClassifier
{
    // Matches "example.com", "example.co.uk", "sub.example.com:8080/path"
    private static readonly Regex DomainRegex =
        new(@"^[a-zA-Z0-9]([a-zA-Z0-9\-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]*[a-zA-Z0-9])?)+(/.*)?(:\d+)?(/.*)?$",
            RegexOptions.Compiled);

    public static InputCategory Classify(string input)
    {
        var trimmed = input.Trim();
        if (string.IsNullOrEmpty(trimmed))
            return InputCategory.Empty;

        // Explicit scheme: http://, https://, file://
        var lower = trimmed.ToLowerInvariant();
        if (lower.StartsWith("http://") || lower.StartsWith("https://") || lower.StartsWith("file://"))
            return InputCategory.Url;

        // "localhost" or "localhost:port"
        if (lower.StartsWith("localhost"))
            return InputCategory.Url;

        // Looks like a domain (e.g., "github.com", "rust-lang.org")
        if (DomainRegex.IsMatch(trimmed))
            return InputCategory.Url;

        return InputCategory.Search;
    }

    /// <summary>
    /// Normalise a URL input so it can be opened by a browser.
    /// If the user typed "github.com" we prepend "https://".
    /// </summary>
    public static string NormalizeUrl(string input)
    {
        var trimmed = input.Trim();
        var lower = trimmed.ToLowerInvariant();
        if (lower.StartsWith("http://") || lower.StartsWith("https://") || lower.StartsWith("file://"))
            return trimmed;
        return "https://" + trimmed;
    }
}

/// <summary>
/// Provides search-engine quick actions (Bing, Google, Baidu).
/// </summary>
public static class SearchEngineService
{
    public sealed record SearchEngine(string Name, string UrlTemplate, string Category = "搜索引擎")
    {
        public string BuildSearchUrl(string query)
            => string.Format(UrlTemplate, Uri.EscapeDataString(query));
    }

    public static readonly SearchEngine[] Engines =
    [
        new("Bing 搜索", "https://www.bing.com/search?q={0}"),
        new("Google 搜索", "https://www.google.com/search?q={0}"),
        new("百度搜索", "https://www.baidu.com/s?wd={0}"),
    ];
}
