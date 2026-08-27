# measure_reopen.ps1 — MainForm 启动→窗口可见延迟测量 (D9 验收工具)
# 用法: powershell -NoProfile -ExecutionPolicy Bypass -File script/measure_reopen.ps1 [-Exe <path>] [-Runs N] [-TitlePrefix "VoidMei 设置"]
# 观测: 进程启动 → EnumWindows 轮询匹配标题前缀的可见窗口 (10ms 步进, 30s 超时)
# 输出: 每次样本 ms + 均值/P95/最大; 首样本 = 真冷启动 (OS 缓存), 其余偏热
param(
    [string]$Exe = "rust/target/debug/voidmei.exe",
    [int]$Runs = 5,
    [string]$TitlePrefix = "VoidMei 设置"
)

$ErrorActionPreference = "Stop"
# stdout 走 UTF-8 (bash tee 存档不乱码)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
if (-not (Test-Path $Exe)) { Write-Error "exe 不存在: $Exe"; exit 2 }

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WinScan {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lp);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    public static bool Found;
    public static string Prefix;
    // 扫描一次: 任一可见窗口标题以 Prefix 开头即命中 (iced/web 两版标题共用前缀)
    public static void Scan() {
        Found = false;
        EnumWindows(delegate(IntPtr h, IntPtr lp) {
            var sb = new StringBuilder(256);
            GetWindowTextW(h, sb, 256);
            if (sb.ToString().StartsWith(Prefix) && IsWindowVisible(h)) { Found = true; return false; }
            return true;
        }, IntPtr.Zero);
    }
}
"@
[WinScan]::Prefix = $TitlePrefix

$samples = @()
for ($i = 1; $i -le $Runs; $i++) {
    # 清残留: 上一轮进程确保退出 (托盘常驻设计, Kill 兜底)
    Get-Process -Name (Split-Path $Exe -Leaf).Replace(".exe","") -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $Exe -PassThru -WorkingDirectory (Get-Location)
    $hit = $false
    while ($sw.ElapsedMilliseconds -lt 30000) {
        [WinScan]::Scan()
        if ([WinScan]::Found) { $hit = $true; break }
        Start-Sleep -Milliseconds 10
    }
    $ms = $sw.ElapsedMilliseconds
    if (-not $hit) {
        Write-Output ("run {0}: TIMEOUT (>30s)" -f $i)
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        exit 1
    }
    Write-Output ("run {0}: {1} ms{2}" -f $i, $ms, $(if ($i -eq 1) { "  (cold)" } else { "" }))
    $samples += $ms
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    # 等子进程树退出 (win32 线程/Service)
    Start-Sleep -Milliseconds 500
}

$sorted = $samples | Sort-Object
$mean = [math]::Round(($samples | Measure-Object -Average).Average, 1)
$p95 = $sorted[[math]::Min([math]::Floor($sorted.Count * 0.95), $sorted.Count - 1)]
$max = $sorted[-1]
Write-Output ""
Write-Output ("样本数 {0}  均值 {1} ms  P95 {2} ms  最大 {3} ms" -f $samples.Count, $mean, $p95, $max)
Write-Output ("首样本 (真冷启动) {0} ms; 最快 {1} ms; 其余为 OS 缓存后的热启动" -f $samples[0], $sorted[0])
