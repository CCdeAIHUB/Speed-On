using System;
using System.Threading.Tasks;

namespace SpeedOnClient.Models;

/// <summary>
/// Represents a single selectable item shown in the search results list.
/// Each item knows how to open itself via the <see cref="OpenAction"/> delegate.
/// </summary>
public sealed class ResultItem
{
    public string Title { get; init; } = string.Empty;
    public string Subtitle { get; init; } = string.Empty;
    public string Category { get; init; } = string.Empty;
    public ItemIconKind IconKind { get; init; }
    public Func<Task> OpenAction { get; init; } = () => Task.CompletedTask;

    public override string ToString() => Title;
}

public enum ItemIconKind
{
    Application,
    File,
    Folder,
    BrowserUrl,
    SearchEngine,
    Browser,
    Default,
}
