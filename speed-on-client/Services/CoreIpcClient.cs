using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;

namespace SpeedOnClient.Services;

/// <summary>
/// Manages the child process <c>speed-on-ipc-stdio</c> and communicates
/// with the Rust core via stdin/stdout JSON Lines.
/// </summary>
public sealed class CoreIpcClient : IDisposable
{
    private const string ProtocolVersion = "speed-on-ipc-v1";

    private Process? _process;
    private StreamWriter? _stdin;
    private StreamReader? _stdout;
    private int _requestCounter;
    private readonly ConcurrentDictionary<string, TaskCompletionSource<JsonElement>> _pending = new();
    private CancellationTokenSource _readCts = new();
    private bool _disposed;

    // --- IPC DTOs (kept minimal; matched to core's Core API v1) ---

    private sealed record IpcRequest(
        [property:JsonPropertyName("protocol_version")] string ProtocolVersion,
        [property:JsonPropertyName("request_id")] string RequestId,
        [property:JsonPropertyName("command")] string Command,
        [property:JsonPropertyName("payload")] JsonElement Payload);

    /// <summary>
    /// Attempt to locate and launch the Rust core binary.
    /// The search order is: SPEED_ON_CORE_PATH env var, sibling directory,
    /// then the workspace target/release folder.
    /// </summary>
    public Task StartAsync()
    {
        var corePath = ResolveCorePath();
        if (corePath == null)
        {
            Debug.WriteLine("Core binary not found — running in degraded mode (no core search).");
            return Task.CompletedTask;
        }

        var dbPath = ResolveDatabasePath();

        var psi = new ProcessStartInfo
        {
            FileName = corePath,
            Arguments = $"--db \"{dbPath}\" --enable-command-opener --enable-application-scan",
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
            StandardOutputEncoding = System.Text.Encoding.UTF8,
        };

        try
        {
            _process = Process.Start(psi);
            if (_process == null) return Task.CompletedTask;

            _stdin = _process.StandardInput;
            _stdin.AutoFlush = false;
            _stdout = _process.StandardOutput;

            // Start background reader loop.
            _ = Task.Run(() => ReadLoop(_readCts.Token));

            // Trigger an initial application scan.
            _ = RefreshApplicationsAsync();
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Failed to start core process: {ex.Message}");
        }

        return Task.CompletedTask;
    }

    // ------ Public API ------

    /// <summary>
    /// Send a <c>search</c> command and return the matching results.
    /// Returns an empty list if the core is unavailable.
    /// </summary>
    public async Task<List<CoreSearchResult>> SearchAsync(string query, int limit = 8)
    {
        if (!IsRunning) return [];

        var payload = new
        {
            query,
            limit,
            now_millis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        };

        var response = await SendCommandAsync("search", payload);
        if (response == null) return [];

        var results = new List<CoreSearchResult>();
        if (response.Value.TryGetProperty("data", out var data) &&
            data.TryGetProperty("results", out var resultsEl) &&
            resultsEl.ValueKind == JsonValueKind.Array)
        {
            foreach (var r in resultsEl.EnumerateArray())
            {
                results.Add(CoreSearchResult.FromJson(r));
            }
        }
        return results;
    }

    /// <summary>
    /// Send a <c>recommend</c> command for the default home-screen list.
    /// </summary>
    public async Task<List<CoreSearchResult>> RecommendAsync(int limit = 8)
    {
        if (!IsRunning) return [];

        var payload = new
        {
            limit,
            now_millis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        };

        var response = await SendCommandAsync("recommend", payload);
        if (response == null) return [];

        var results = new List<CoreSearchResult>();
        if (response.Value.TryGetProperty("data", out var data) &&
            data.TryGetProperty("results", out var resultsEl) &&
            resultsEl.ValueKind == JsonValueKind.Array)
        {
            foreach (var r in resultsEl.EnumerateArray())
            {
                results.Add(CoreSearchResult.FromJson(r));
            }
        }
        return results;
    }

    /// <summary>
    /// Open a resource through the core's <c>open_resource</c> command
    /// so the activity is recorded for future recommendations.
    /// </summary>
    public async Task<bool> OpenResourceAsync(CoreSearchResult resource)
    {
        if (!IsRunning) return false;

        var payload = new
        {
            resource = new
            {
                id = resource.Id,
                kind = resource.Kind,
                title = resource.Title,
                target = resource.Target,
                icon_path = (string?)null,
            },
            requested_at_millis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        };

        var response = await SendCommandAsync("open_resource", payload);
        return response != null &&
               response.Value.TryGetProperty("data", out var data) &&
               data.TryGetProperty("opened", out var opened) &&
               opened.GetBoolean();
    }

    /// <summary>
    /// Trigger the core to rescan installed applications.
    /// </summary>
    public async Task RefreshApplicationsAsync()
    {
        if (!IsRunning) return;

        var payload = new
        {
            requested_at_millis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
        };
        await SendCommandAsync("refresh_applications", payload);
    }

    public bool IsRunning => _process != null && !_process.HasExited;

    // ------ Internal plumbing ------

    private async Task<JsonElement?> SendCommandAsync(string command, object payload)
    {
        if (_stdin == null) return null;

        var requestId = Interlocked.Increment(ref _requestCounter).ToString();
        var tcs = new TaskCompletionSource<JsonElement>(TaskCreationOptions.RunContinuationsAsynchronously);
        _pending[requestId] = tcs;

        var request = new IpcRequest(ProtocolVersion, requestId, command, JsonSerializer.SerializeToElement(payload));
        var json = JsonSerializer.Serialize(request);

        try
        {
            await _stdin.WriteLineAsync(json);
            await _stdin.FlushAsync();
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Failed to send IPC request: {ex.Message}");
            _pending.TryRemove(requestId, out _);
            return null;
        }

        // Timeout after 5 seconds.
        using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        cts.Token.Register(() => tcs.TrySetCanceled());

        try
        {
            var root = await tcs.Task;
            // The root element is the full IpcResponse.
            if (root.TryGetProperty("response", out var response) &&
                response.TryGetProperty("ok", out var ok) &&
                ok.GetBoolean())
            {
                return response;
            }
            return response;
        }
        catch (OperationCanceledException)
        {
            _pending.TryRemove(requestId, out _);
            return null;
        }
    }

    private async Task ReadLoop(CancellationToken ct)
    {
        if (_stdout == null) return;

        while (!ct.IsCancellationRequested)
        {
            string? line;
            try
            {
                line = await _stdout.ReadLineAsync(ct);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Core stdout read error: {ex.Message}");
                break;
            }

            if (line == null) break; // Process exited
            if (string.IsNullOrWhiteSpace(line)) continue;

            try
            {
                using var doc = JsonDocument.Parse(line);
                var root = doc.RootElement;
                if (root.TryGetProperty("request_id", out var idEl))
                {
                    var requestId = idEl.GetString();
                    if (requestId != null && _pending.TryRemove(requestId, out var tcs))
                    {
                        tcs.TrySetResult(root.Clone());
                    }
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Failed to parse IPC response: {ex.Message}");
            }
        }
    }

    private static string? ResolveCorePath()
    {
        // 1. Environment variable
        var envPath = Environment.GetEnvironmentVariable("SPEED_ON_CORE_PATH");
        if (!string.IsNullOrEmpty(envPath) && File.Exists(envPath))
            return envPath;

        var exeDir = AppContext.BaseDirectory;
        var os = OperatingSystem.IsWindows() ? "speed-on-ipc-stdio.exe" : "speed-on-ipc-stdio";

        // 2. Sibling to the client executable
        var sibling = Path.Combine(exeDir, os);
        if (File.Exists(sibling)) return sibling;

        // 3. Workspace target/release (development)
        var devPath = Path.Combine(exeDir, "..", "..", "..", "..", "target", "release", os);
        if (File.Exists(devPath)) return Path.GetFullPath(devPath);

        return null;
    }

    private static string ResolveDatabasePath()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "SpeedOn");
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, "speed-on.db");
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _readCts.Cancel();

        try
        {
            if (_stdin != null)
            {
                _stdin.Close();
            }
            if (_process != null && !_process.HasExited)
            {
                _process.Kill(entireProcessTree: true);
                _process.WaitForExit(3000);
            }
        }
        catch { /* best effort */ }

        _process?.Dispose();
        _readCts.Dispose();
    }
}

/// <summary>
/// A search/recommend result item returned by the Rust core.
/// </summary>
public sealed class CoreSearchResult
{
    public string Id { get; init; } = string.Empty;
    public string Kind { get; init; } = string.Empty;
    public string Title { get; init; } = string.Empty;
    public string Target { get; init; } = string.Empty;
    public string? IconPath { get; init; }
    public long Score { get; init; }
    public string MatchKind { get; init; } = string.Empty;
    public string Reason { get; init; } = string.Empty;

    public static CoreSearchResult FromJson(JsonElement el)
    {
        var result = new CoreSearchResult();
        if (el.TryGetProperty("resource", out var res))
        {
            return new CoreSearchResult
            {
                Id = res.TryGetProperty("id", out var id) ? id.GetString() ?? "" : "",
                Kind = res.TryGetProperty("kind", out var kind) ? kind.GetString() ?? "" : "",
                Title = res.TryGetProperty("title", out var title) ? title.GetString() ?? "" : "",
                Target = res.TryGetProperty("target", out var target) ? target.GetString() ?? "" : "",
                IconPath = res.TryGetProperty("icon_path", out var icon) && icon.ValueKind == JsonValueKind.String ? icon.GetString() : null,
                Score = el.TryGetProperty("score", out var score) ? score.GetInt64() : 0,
                MatchKind = el.TryGetProperty("match_kind", out var mk) ? mk.GetString() ?? "" : "",
                Reason = el.TryGetProperty("reason", out var reason) ? reason.GetString() ?? "" : "",
            };
        }
        return result;
    }
}
