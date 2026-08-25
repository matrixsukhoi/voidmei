#!/usr/bin/env bash
# Rust ↔ Java FlightInfoOverlay 渲染对拍 (M1 验收入口)
# 流程: Java 离屏导出 → 读 java meta 的 numHeight 注入 Rust → 双 PNG compare + meta 硬断言
# 产物: build/rust_ref/{java,rust}_preview.png + diff.png (热力图, 供人工审)
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
echo "完成: 热力图 $OUT/diff.png 供人工审 (R=RGB 差, G=alpha 差, 黑=一致)"
