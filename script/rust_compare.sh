#!/usr/bin/env bash
# Rust ↔ Java FlightInfoOverlay 渲染对拍 (M1 验收入口)
# 流程: Java 离屏导出 → 读 java meta 的 numHeight 注入 Rust → 双 PNG compare + meta 硬断言
#       + gauge 段: linear/compass/attitude 三组件像素基线 (D7 验收, 尽力而为)
#       + minihud 段: 默认配置完整 HUD 整帧 (preview 静态数据, D7 验收)
# 产物: build/rust_ref/{java,rust}_preview.png + diff.png (热力图, 供人工审)
#       build/rust_ref/{java,rust}_gauge_*.png + diff_gauge_*.png
#       build/rust_ref/{java,rust}_minihud.png + diff_minihud.png
# 需求: 桌面环境 (Java Toolkit 字体度量)、cargo、JDK 8
# 注意: 全程相对路径 (git-bash 的 /c/... 绝对路径 Windows java/python 不识别)
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUTREL="build/rust_ref"
OUT="$ROOT/$OUTREL"
mkdir -p "$OUT"

# classpath 分隔符: Windows ; / 其余 :
case "$OSTYPE" in
  msys*|cygwin*|win32*) SEP=";" ;;
  *) SEP=":" ;;
esac
CP="bin${SEP}dep/*"

echo "[1/5] 编译 Java 并导出离屏 PNG ..."
cd "$ROOT"
python script/build.py compile
java -classpath "$CP" ui.debug.OverlayPngExport \
  --out "$OUTREL/java_preview.png" --meta "$OUTREL/java_meta.json" > /dev/null

echo "[2/5] 读取 Java numHeight (校准值) ..."
NH=$(cd "$ROOT" && python -c "import json;print(json.load(open(r'$OUTREL/java_meta.json'))['num_height'])")
echo "      num_height=$NH"

echo "[3/5] 构建 Rust 并导出离屏 PNG ..."
cd "$ROOT/rust"
cargo build --release
./target/release/voidmei-overlay --render-png "../$OUTREL/rust_preview.png" \
  --meta "../$OUTREL/rust_meta.json" --num-height "$NH"

echo "[4/5] meta 硬断言 (布局整数运算必须完全一致) ..."
python - "$OUTREL" <<'EOF'
import json, os, sys
out = os.path.join(os.pardir, sys.argv[1])  # cwd=rust/, meta 在 repo 根下
a = json.load(open(os.path.join(out, 'java_meta.json')))
b = json.load(open(os.path.join(out, 'rust_meta.json')))
keys = ['font_size', 'label_font_size', 'unit_font_size', 'column_num',
        'num_height', 'total_width', 'total_height', 'visible_fields', 'aa']
bad = [(k, a[k], b[k]) for k in keys if a[k] != b[k]]
if bad:
    print('FAIL meta 不一致: %s' % bad)
    sys.exit(1)
print('      PASS (布局度量完全一致)')
EOF

echo "[5/5] 像素比对 (尽力而为 + 人工审) ..."
./target/release/voidmei-overlay compare "../$OUTREL/java_preview.png" "../$OUTREL/rust_preview.png" \
  --heatmap "../$OUTREL/diff.png"
echo ""

# ---- gauge 对拍段 (D7: 三 gauge 组件像素基线, 默认数据) ----
# 常量表同源: java ui.debug.OverlayPngExport exportGauge* ↔ rust parity_gauges.rs
echo "[gauge] 三 gauge 组件对拍 (linear/compass/attitude, 默认数据) ..."
cd "$ROOT"
for G in linear compass attitude; do
  echo "  ---- gauge: $G ----"
  java -classpath "$CP" ui.debug.OverlayPngExport --gauge "$G" --out "$OUTREL/java_gauge_$G.png" > /dev/null
  "$ROOT/rust/target/release/voidmei-overlay" --gauge "$G" --out "$OUTREL/rust_gauge_$G.png"
  "$ROOT/rust/target/release/voidmei-overlay" compare "$OUTREL/java_gauge_$G.png" "$OUTREL/rust_gauge_$G.png" \
    --heatmap "$OUTREL/diff_gauge_$G.png"
done
echo ""

# ---- minihud 整帧对拍段 (D7: 默认配置完整 HUD, preview 静态数据注入) ----
# 组装同源: java ui.debug.OverlayPngExport exportMiniHud (MiniHUDOverlay.init 编排快照)
#           ↔ rust parity_minihud.rs (MiniHudOverlay::init 生产编排)
# 已知差异预期: row2 (Java HUDMechanizationRow 三段占位 vs Rust HUDFlapsRow 合并串占位,
# rows.rs 模块头备案) → row2 文字位 + 挂其右缘的 attitude/compass 横向 ~1 字符格偏移;
# 其余区域应仅剩 AA 光栅化差异 (口径同 gauge 段)。
echo "[minihud] MiniHUD 整帧对拍 (默认配置, preview 数据) ..."
java -classpath "$CP" ui.debug.OverlayPngExport --minihud --out "$OUTREL/java_minihud.png" > /dev/null
"$ROOT/rust/target/release/voidmei-overlay" --minihud --out "$OUTREL/rust_minihud.png"
"$ROOT/rust/target/release/voidmei-overlay" compare "$OUTREL/java_minihud.png" "$OUTREL/rust_minihud.png" \
  --heatmap "$OUTREL/diff_minihud.png"
echo ""
echo "完成: 热力图 $OUT/diff.png + $OUT/diff_gauge_*.png + $OUT/diff_minihud.png 供人工审 (R=RGB 差, G=alpha 差, 黑=一致)"
