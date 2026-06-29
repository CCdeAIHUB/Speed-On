using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using SpeedOnClient.Models;
using SpeedOnClient.Services;

namespace SpeedOnClient.ViewModels;

/// <summary>
/// Orchestrates the search pipeline:
/// 1. Classify input (URL / search / empty).
/// 2. Query the Rust core for results (when applicable).
/// 3. Merge core results with search-engine / browser options.
/// 4. Expose an observable list of <see cref="ResultItem"/>s for the UI.
/// </summary>
public sealed class MainViewModel
{
    private readonly CoreIpcClient _core;
    private readonly BrowserDetector _browserDetector = new();
    private CancellationTokenSource _searchCts = new();

    public ObservableCollection<ResultItem> Results { get; } = [];

    public async Task UpdateResultsAsync(string input)
    {
        // Cancel any in-flight search.
        _searchCts.Cancel();
        _searchCts = new CancellationTokenSource();
        var ct = _searchCts.Token;

        Results.Clear();

        var category = InputClassifier.Classify(input);

        switch (category)
        {
            case InputCategory.Empty:
                await LoadRecommendationsAsync(ct);
                break;

            case InputCategory.Url:
                await LoadUrlResultsAsync(input, ct);
                break;

            case InputCategory.Search:
                await LoadSearchResultsAsync(input, ct);
                break;
        }
    }

    private async Task LoadRecommendationsAsync(CancellationToken ct)
    {
        // Show core recommendations (most used apps).
        var recommendations = await _core.RecommendAsync(limit: 8);
        if (ct.IsCancellationRequested) return;

        foreach (var r in recommendations)
        {
            var result = r;
            Results.Add(new ResultItem
            {
                Title = r.Title,
                Subtitle = r.Target,
                Category = "推荐",
                IconKind = MapIconKind(r.Kind),
                OpenAction = async () => await _core.OpenResourceAsync(result),
            });
        }

        if (Results.Count == 0)
        {
            Results.Add(new ResultItem
            {
                Title = "开始输入以搜索...",
                Subtitle = "搜索应用、文件、网址",
                Category = "提示",
                IconKind = ItemIconKind.Default,
            });
        }
    }

    private async Task LoadUrlResultsAsync(string input, CancellationToken ct)
    {
        var url = InputClassifier.NormalizeUrl(input);
        var browsers = _browserDetector.DetectBrowsers();

        foreach (var browser in browsers)
        {
            var browserPath = browser.ExecutablePath;
            var targetUrl = url;
            Results.Add(new ResultItem
            {
                Title = $"在 {browser.Name} 中打开",
                Subtitle = url,
                Category = browser.IsDefault ? "默认浏览器" : "浏览器",
                IconKind = ItemIconKind.Browser,
                OpenAction = () =>
                {
                    ResourceOpener.OpenUrlInBrowser(browserPath, targetUrl);
                    return Task.CompletedTask;
                },
            });
        }

        // Fallback: open with system default
        if (browsers.Count == 0)
        {
            var targetUrl = url;
            Results.Add(new ResultItem
            {
                Title = "在默认浏览器中打开",
                Subtitle = url,
                Category = "浏览器",
                IconKind = ItemIconKind.Browser,
                OpenAction = () =>
                {
                    ResourceOpener.OpenUrlDefault(targetUrl);
                    return Task.CompletedTask;
                },
            });
        }
    }

    private async Task LoadSearchResultsAsync(string input, CancellationToken ct)
    {
        // 1. Query the core for local matches (apps, files, etc.)
        var coreResults = await _core.SearchAsync(input, limit: 8);
        if (ct.IsCancellationRequested) return;

        foreach (var r in coreResults)
        {
            var result = r;
            Results.Add(new ResultItem
            {
                Title = r.Title,
                Subtitle = r.Target,
                Category = MapCategory(r.Kind, r.MatchKind),
                IconKind = MapIconKind(r.Kind),
                OpenAction = async () => await _core.OpenResourceAsync(result),
            });
        }

        // 2. Append search-engine quick actions.
        foreach (var engine in SearchEngineService.Engines)
        {
            var query = input;
            var eng = engine;
            var searchUrl = engine.BuildSearchUrl(query);
            Results.Add(new ResultItem
            {
                Title = $"{eng.Name}: \"{query}\"",
                Subtitle = searchUrl,
                Category = "搜索引擎",
                IconKind = ItemIconKind.SearchEngine,
                OpenAction = () =>
                {
                    ResourceOpener.OpenSearchEngine(searchUrl);
                    return Task.CompletedTask;
                },
            });
        }
    }

    private static ItemIconKind MapIconKind(string kind) => kind switch
    {
        "application" => ItemIconKind.Application,
        "file" => ItemIconKind.File,
        "folder" => ItemIconKind.Folder,
        "browser_url" => ItemIconKind.BrowserUrl,
        _ => ItemIconKind.Default,
    };

    private static string MapCategory(string kind, string matchKind)
    {
        var kindLabel = kind switch
        {
            "application" => "应用",
            "file" => "文件",
            "folder" => "文件夹",
            "browser_url" => "网页",
            _ => "其他",
        };
        return matchKind == "user_history" ? $"历史 · {kindLabel}" : kindLabel;
    }

    public MainViewModel(CoreIpcClient core)
    {
        _core = core;
    }
}
