using System;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Threading;
using SpeedOnClient.Models;
using SpeedOnClient.Services;
using SpeedOnClient.ViewModels;

namespace SpeedOnClient.Views;

public partial class MainWindow : Window
{
    private readonly MainViewModel _viewModel;

    public MainWindow() : this(null) { }

    public MainWindow(CoreIpcClient? coreClient)
    {
        InitializeComponent();

        _viewModel = new MainViewModel(coreClient ?? new CoreIpcClient());
        ResultsList.ItemsSource = _viewModel.Results;

        SearchInput.TextChanged += OnSearchInputTextChanged;
        ResultsList.SelectionChanged += OnSelectionChanged;

        // Hide the window initially — it's shown via hotkey or tray.
        Hide();
    }

    // --- Window lifecycle ---

    public void ShowWindow()
    {
        PositionAtTopCenter();
        Show();
        Activate();
        SearchInput.Text = string.Empty;
        SearchInput.Focus();

        // Load recommendations for the empty state.
        _ = _viewModel.UpdateResultsAsync(string.Empty);
        if (_viewModel.Results.Count > 0)
            ResultsList.SelectedIndex = 0;
    }

    public void HideWindow()
    {
        SearchInput.Text = string.Empty;
        _viewModel.Results.Clear();
        Hide();
    }

    private void PositionAtTopCenter()
    {
        var screen = Screens.ScreenFromPoint(Position) ?? Screens.Primary;
        if (screen == null) return;

        var workArea = screen.WorkingArea;
        var x = workArea.X + (workArea.Width - Width) / 2;
        var y = workArea.Y + 40; // 40px from top
        Position = new PixelPoint((int)x, (int)y);
    }

    // --- Search input handling ---

    private async void OnSearchInputTextChanged(object? sender, TextChangedEventArgs e)
    {
        var text = SearchInput.Text ?? string.Empty;
        await _viewModel.UpdateResultsAsync(text);

        // Auto-select first result.
        DispatcherTimer.RunOnce(() =>
        {
            if (ResultsList.ItemCount > 0)
                ResultsList.SelectedIndex = 0;
        }, TimeSpan.FromMilliseconds(50));
    }

    private void OnSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        // Scroll selected item into view.
        if (ResultsList.SelectedItem != null)
            ResultsList.ScrollIntoView(ResultsList.SelectedItem);
    }

    // --- Keyboard navigation ---

    private void SearchInput_KeyDown(object? sender, KeyEventArgs e)
    {
        switch (e.Key)
        {
            case Key.Escape:
                HideWindow();
                e.Handled = true;
                break;

            case Key.Down:
            case Key.Right:
                if (ResultsList.ItemCount > 0)
                {
                    ResultsList.SelectedIndex = Math.Min(ResultsList.SelectedIndex + 1, ResultsList.ItemCount - 1);
                    ResultsList.Focus();
                }
                e.Handled = true;
                break;

            case Key.Up:
            case Key.Left:
                if (ResultsList.ItemCount > 0)
                {
                    ResultsList.SelectedIndex = Math.Max(ResultsList.SelectedIndex - 1, 0);
                    ResultsList.Focus();
                }
                e.Handled = true;
                break;

            case Key.Enter:
                if (ResultsList.SelectedItem is ResultItem item)
                {
                    _ = OpenAndHideAsync(item);
                }
                e.Handled = true;
                break;
        }
    }

    private async Task OpenAndHideAsync(ResultItem item)
    {
        try
        {
            await item.OpenAction();
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Open action failed: {ex.Message}");
        }
        HideWindow();
    }

    // --- Deactivate handling (click outside to hide) ---

    protected override void OnLostFocus(RoutedEventArgs e)
    {
        base.OnLostFocus(e);
        // Hide when the window loses focus (user clicked elsewhere).
        DispatcherTimer.RunOnce(() =>
        {
            if (!IsActive && IsVisible)
                HideWindow();
        }, TimeSpan.FromMilliseconds(100));
    }

    // Avalonia 11 Window does not have a virtual OnDeactivated; subscribe to event instead.
    protected override void OnOpened(EventArgs e)
    {
        base.OnOpened(e);
        Deactivated += (_, _) =>
        {
            if (IsVisible)
                HideWindow();
        };
    }
}
