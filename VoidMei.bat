@echo off
setlocal enabledelayedexpansion

REM VoidMei 智能启动脚本
REM 优先查找 Java 8，支持回退到其他版本
REM 支持的 Java 8 发行版: Oracle, Eclipse Temurin, Azul Zulu, Amazon Corretto, Microsoft OpenJDK

set JAVA_EXE=
set JAVA8_FOUND=0

REM === 步骤 1: 搜索 Registry 中的 Java 8 ===

REM 1.1 Oracle JRE 1.8
for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\JavaSoft\Java Runtime Environment\1.8" /v JavaHome 2^>nul') do (
    if exist "%%b\bin\java.exe" (
        set JAVA_EXE=%%b\bin\java.exe
        set JAVA8_FOUND=1
        echo [VoidMei] 找到 Java 8 (Oracle JRE): %%b
    )
)

REM 1.2 Oracle JDK 1.8
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\JavaSoft\JDK\1.8" /v JavaHome 2^>nul') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Oracle JDK): %%b
        )
    )
)

REM 1.3 Eclipse Temurin JRE 8 (原 AdoptOpenJDK)
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Eclipse Adoptium\JRE\8\hotspot" /s /v Path 2^>nul ^| findstr /i "REG_SZ"') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Eclipse Temurin): %%b
        )
    )
)

REM 1.4 Eclipse Temurin JDK 8
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Eclipse Adoptium\JDK\8\hotspot" /s /v Path 2^>nul ^| findstr /i "REG_SZ"') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Eclipse Temurin JDK): %%b
        )
    )
)

REM 1.5 Azul Zulu 8
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Azul Systems\Zulu\zulu-8" /v InstallationPath 2^>nul') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Azul Zulu): %%b
        )
    )
)

REM 1.6 Amazon Corretto 8
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Amazon.com\Corretto\8" /s /v Location 2^>nul ^| findstr /i "REG_SZ"') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Amazon Corretto): %%b
        )
    )
)

REM 1.7 Microsoft OpenJDK 8 (Build of OpenJDK)
if !JAVA8_FOUND!==0 (
    for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Microsoft\JDK\8" /s /v Path 2^>nul ^| findstr /i "REG_SZ"') do (
        if exist "%%b\bin\java.exe" (
            set JAVA_EXE=%%b\bin\java.exe
            set JAVA8_FOUND=1
            echo [VoidMei] 找到 Java 8 (Microsoft OpenJDK): %%b
        )
    )
)

REM === 步骤 2: 检查 JAVA_HOME (可能是 Java 8) ===
if !JAVA8_FOUND!==0 (
    if defined JAVA_HOME (
        if exist "%JAVA_HOME%\bin\java.exe" (
            echo [VoidMei] 使用 JAVA_HOME: %JAVA_HOME%
            echo [VoidMei] 警告: 未在 Registry 找到 Java 8，使用 JAVA_HOME 中的 Java
            set JAVA_EXE=%JAVA_HOME%\bin\java.exe
        )
    )
)

REM === 步骤 3: 回退到 PATH ===
if not defined JAVA_EXE (
    where java >nul 2>&1
    if !errorlevel!==0 (
        echo [VoidMei] 使用 PATH 中的 java
        echo [VoidMei] 警告: 未找到 Java 8，使用 PATH 中的 Java (可能不兼容)
        set JAVA_EXE=java
    )
)

REM === 启动 VoidMei ===
if not defined JAVA_EXE (
    echo.
    echo [VoidMei] 错误：未找到 Java 运行环境
    echo.
    echo 请安装 Java 8，推荐以下任一版本：
    echo   - Eclipse Temurin 8: https://adoptium.net/temurin/releases/?version=8
    echo   - Azul Zulu 8: https://www.azul.com/downloads/?version=java-8-lts
    echo   - Amazon Corretto 8: https://aws.amazon.com/corretto/
    echo.
    pause
    exit /b 1
)

REM === 显示版本并启动 ===
echo [VoidMei] 启动中...
"!JAVA_EXE!" -Dsun.java2d.uiScale=1 -Xms64m -Xmx320m -jar VoidMei.jar %*
