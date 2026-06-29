<#
.SYNOPSIS
    VoidMei Windows PowerShell构建脚本
.DESCRIPTION
    编译Java源码 → 打包JAR → 生成EXE(launch4j)
    复制和运行由 Downloads/VoidMei/voidmei.ps1 负责
.NOTES
    需要 JDK 1.8 环境
    如果未安装 launch4j，EXE生成步骤会自动跳过
#>

$ErrorActionPreference = "Stop"

# 切换到项目根目录（script/ 的上级目录）
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

Write-Host "=== VoidMei 构建脚本 (PowerShell) ===" -ForegroundColor Yellow
Write-Host "项目目录: $ProjectRoot" -ForegroundColor Cyan
Write-Host ""

# ============================================================
# 第1步：编译 Java 源码
# ============================================================
Write-Host "[1/3] 编译 Java 源码..." -ForegroundColor Yellow

# 清理旧的 class 文件
if (Test-Path bin) {
    Remove-Item -Recurse -Force bin
    Write-Host "  清理 bin/ 目录"
}
New-Item -ItemType Directory -Force -Path bin | Out-Null

# 收集所有 .java 源文件（用 WriteAllLines 避免 Out-File BOM 问题）
$javaSources = Get-ChildItem -Path src -Recurse -Filter "*.java" | ForEach-Object { $_.FullName }
[System.IO.File]::WriteAllLines("$pwd\sources.txt", $javaSources)
$javaFileCount = $javaSources.Count
Write-Host "  找到 $javaFileCount 个 Java 源文件"

# 编译
$javacArgs = @("-encoding", "UTF-8", "-d", "bin", "-classpath", "dep\*", "@sources.txt")
$javacProcess = Start-Process -FilePath "javac" -ArgumentList $javacArgs -Wait -NoNewWindow -PassThru

# 清理 sources.txt
Remove-Item -Force sources.txt -ErrorAction SilentlyContinue

if ($javacProcess.ExitCode -ne 0) {
    Write-Host "  [错误] 编译失败！" -ForegroundColor Red
    exit 1
}
Write-Host "  编译完成 ✅" -ForegroundColor Green
Write-Host ""

# ============================================================
# 第2步：打包 JAR
# ============================================================
Write-Host "[2/3] 打包 JAR..." -ForegroundColor Yellow

$jarArgs = @("-cvfm", "VoidMei.jar", "MANIFEST.MF", "-C", "bin", ".")
$jarProcess = Start-Process -FilePath "jar" -ArgumentList $jarArgs -Wait -NoNewWindow -PassThru

if ($jarProcess.ExitCode -ne 0) {
    Write-Host "  [错误] JAR打包失败！" -ForegroundColor Red
    exit 1
}
Write-Host "  打包完成 ✅" -ForegroundColor Green
Write-Host ""

# ============================================================
# 第3步：生成 Windows EXE (launch4j)
# ============================================================
Write-Host "[3/3] 生成 Windows EXE (launch4j)..." -ForegroundColor Yellow

# 查找 launch4j：先搜 PATH，再搜常见安装目录
$launch4jExe = $null
$launch4jNames = @("launch4j", "launch4jc")
$launch4jDirs = @(
    ${env:ProgramFiles(x86)}, ${env:ProgramFiles}
)

# 1) PATH 中查找
foreach ($name in $launch4jNames) {
    if (Get-Command $name -ErrorAction SilentlyContinue) {
        $launch4jExe = $name
        Write-Host "  在 PATH 中找到: $name" -ForegroundColor DarkGray
        break
    }
}

# 2) 常见安装目录中查找
if (-not $launch4jExe) {
    foreach ($dir in $launch4jDirs) {
        if (-not $dir) { continue }
        $l4jDir = Join-Path $dir "Launch4j"
        foreach ($name in $launch4jNames) {
            $l4jPath = Join-Path $l4jDir "$name.exe"
            if (Test-Path $l4jPath) {
                $launch4jExe = $l4jPath
                Write-Host "  在安装目录中找到: $l4jPath" -ForegroundColor DarkGray
                break
            }
        }
        if ($launch4jExe) { break }
    }
}

if ($launch4jExe) {
    $l4jConfig = Join-Path $ScriptDir "voidmeil4j.xml"
    $l4jProcess = Start-Process -FilePath $launch4jExe -ArgumentList $l4jConfig -Wait -NoNewWindow -PassThru
    if ($l4jProcess.ExitCode -eq 0) {
        Write-Host "  EXE生成完成 ✅" -ForegroundColor Green
    } else {
        Write-Host "  [警告] launch4j执行失败，跳过EXE生成" -ForegroundColor Yellow
    }
} else {
    Write-Host "  [跳过] 未安装 launch4j，仅保留 JAR 包" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "=== 构建流程完成 ===" -ForegroundColor Green
Write-Host "  运行方式: cd ~/Downloads/VoidMei ; .\voidmei.ps1" -ForegroundColor Cyan
