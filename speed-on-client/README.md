# Speed-On Client

C# / Avalonia 前端客户端，通过 stdio JSON Lines IPC 与 Rust 核心通信。

## 功能

- **全局热键 Win+Alt**：随时唤出搜索窗口
- **Spotlight 风格窗口**：无边框、半透明、屏幕顶部居中、失焦自动隐藏
- **智能输入分类**：
  - 网址 → 显示已安装浏览器（默认浏览器优先）
  - 搜索文本 → 核心搜索结果 + Bing / Google / 百度快速搜索
  - 空输入 → 核心推荐（最常用应用）
- **键盘导航**：↑↓ 选择，Enter 打开，Esc 关闭
- **系统托盘**：后台运行，右键菜单（显示窗口 / 刷新索引 / 退出）

## 技术栈

| 组件 | 技术 |
|------|------|
| UI 框架 | Avalonia UI 11.2（跨平台：Windows + macOS） |
| 运行时 | .NET 9 |
| 后端通信 | stdio JSON Lines IPC（子进程） |
| 全局热键 | Windows: WH_KEYBOARD_LL 低级钩子 / macOS: CGEventTap（待实现） |

## 项目结构

```
speed-on-client/
├── Program.cs              # 入口
├── App.axaml / .cs         # 应用生命周期、托盘、热键
├── Views/
│   ├── MainWindow.axaml    # 搜索窗口 UI
│   └── MainWindow.axaml.cs # 窗口逻辑（定位、键盘、失焦）
├── ViewModels/
│   └── MainViewModel.cs    # 搜索管线（分类→查询→合并结果）
├── Models/
│   └── ResultItem.cs       # 结果项模型
└── Services/
    ├── CoreIpcClient.cs     # Rust 核心 IPC 客户端
    ├── GlobalHotkeyService.cs # Win+Alt 全局热键
    ├── BrowserDetector.cs   # 浏览器检测
    ├── InputClassifier.cs   # 输入分类（URL/搜索/空）
    └── ResourceOpener.cs    # 资源打开
```

## 编译

```bash
cd speed-on-client
dotnet build -c Release
```

## 运行

### 前置条件

需要 Rust 核心 `speed-on-ipc-stdio` 二进制文件。放置位置（按优先顺序）：

1. 环境变量 `SPEED_ON_CORE_PATH` 指向的路径
2. 与客户端 exe 同目录
3. `../../target/release/speed-on-ipc-stdio`（开发模式）

### 编译 Rust 核心

```bash
# 在仓库根目录
cargo build --release -p speed-on-ipc-stdio
```

核心二进制会生成在 `target/release/speed-on-ipc-stdio[.exe]`。

### 启动

```bash
dotnet run --project speed-on-client
```

或直接运行编译后的 exe。

## 跨平台说明

- **Windows**：完整支持（热键 + 托盘 + 浏览器检测）
- **macOS**：托盘和 UI 已支持；全局热键需要辅助功能权限（CGEventTap 待实现，当前可通过托盘图标激活）
- **Linux**：UI 已支持；浏览器检测已支持
