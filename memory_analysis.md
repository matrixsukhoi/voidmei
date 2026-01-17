# VoidMei 内存占用深度分析与建议 (Memory Analysis & Optimization)

根据对当前代码库的分析，VoidMei 在运行过程中达到 0.4GB (约 400MB) 的内存占用是多种因素共同作用的结果。对于一个基于 Java 8 和 WebLaF 的 Swing 应用，这虽在“正常”波动范围内，但确实存在显著优化空间。

---

## 1. 核心问题定位 (Problem Identification)

### 1.1 Blkx 原始数据常驻内存
*   **现象**: `Blkx` 类在解析气动文件时，会将整个文件内容读入一个巨大的 `String data` 字段中。
*   **问题**: `getdouble` 和 `getone` 等方法依赖于对该全局字符串的频繁 `indexOf` 和 `substring` 操作。解析完成后，这个原始字符串仍保留在 `Blkx` 对象中，只要当前飞机不更换，这几兆甚至更多的原始文本就会一直占用堆内存。

### 1.2 FMDataOverlay 的对象碎裂
*   **现象**: `FMDataOverlay` 为了显示 FM 数据，会将 `fmdata` 字符串按 `\n` 切分。
*   **问题**: `Arrays.asList(text.split("\n"))` 会创建数千个短小的 `String` 对象。每个 Java String 对象本身有 24-32 字节的固定开销，再加上其内部的 `char[]`。对于一个复杂的 FM 文件，这会瞬间吃掉 10-30MB 的内存，且增加了 GC 的压力。

### 1.3 Service 线程的高频对象创建
*   **现象**: `Service` 在其生命周期内（每 10-80ms 一次循环）频繁进行字符串格式化和 Map 更新。
*   **问题**: 频繁调用 `String.format` 和生成 `FlightDataEvent` 引发的临时对象创建。虽然 JVM 能处理，但在未进行 Minor GC 前，堆占用会持续攀升，直至触发 GC 阈值。

### 1.4 WebLookAndFeel 的重量级渲染
*   **现象**: WebLaF 提供了精美的 UI，但也引入了庞大的样式缓存和皮肤纹理。
*   **问题**: WebLaF 默认会缓存大量的渲染元数据，这在复杂 UI 场景下可能占去 100MB+ 的空间。

---

## 2. 优化建议方案 (Optimization Strategies)

我们建议分阶段实施以下优化：

### 方案 A：Blkx 生命周期管理 (精简数据)
*   **操作**: 在 `Controller.ensureBlkxLoaded()` 完成解析后，调用 `Blkx.clearRawData()` 手动将 `data` 和 `fmdata` 字段置为 `null`（除了 FMDataOverlay 需要用的那份）。
*   **收益**: 释放解析过程中使用的原始文本缓存，节约 5-20MB 内存。

### 方案 B：FMDataOverlay 懒加载 (减少对象数)
*   **操作**: 不要预先 `split("\n")`。将 `fmdata` 保存为单一原始字符串。只有在 UI 渲染器 (`ZebraListRenderer`) 需要按行循环时，才使用 `Scanner` 或手写的行迭代器，避免创建数千个中间 String 对象。
*   **收益**: 显著降低由于 String 碎裂导致的 Minor GC 频率。

### 方案 C：显式 GC 诱导
*   **操作**: 在切换飞机 (Aircraft Swap) 或关闭主配置窗口时，调用 `System.gc()`。
*   **收益**: 主动将不再使用的旧飞机数据（Blkx, Cached Strings）回收，保持内存平稳。

### 方案 D：JVM 参数调优
*   **操作**: 在 `build.sh` 或启动脚本中限制堆大小：`-Xms128m -Xmx320m`。
*   **收益**: 强制 JVM 更加积极地回收内存，防止其无限制地向操作系统申请空间。

---

## 3. 下一步行动建议

1.  **验证**: 首先确认是否为“内存泄漏”（即内存随时间不断增长且不下降）。如果内存稳定在 400MB 不再增长，那更多是 Java 8 默认的弹性堆管理策略。
2.  **实施**: 我可以优先为您实现“Blkx 解析后清除原始数据”和“FMDataOverlay 渲染逻辑优化”这两个不需要改变用户脚本的改动。
