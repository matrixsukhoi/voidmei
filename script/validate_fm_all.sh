#!/bin/bash
# FM 数据全量验证脚本
# 编译项目源码和验证器，然后遍历所有中央配置文件验证 FM 加载
#
# 用法:
#   ./script/validate_fm_all.sh                                    # 使用默认 data 目录
#   ./script/validate_fm_all.sh ~/downloads/voidmei/data/aces/gamedata/flightmodels  # 指定 data 目录
#   DATA_DIR=~/my_data ./script/validate_fm_all.sh                  # 通过环境变量指定
#
# 退出码: 0 = 全部通过, 1 = 有失败, 2 = 环境错误

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 项目根目录
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# 平台检测: Windows Git Bash 用分号，Linux/macOS 用冒号
case "$(uname -s 2>/dev/null || echo Windows)" in
    CYGWIN*|MINGW*|MSYS*|Windows) CPSEP=';' ;;
    *) CPSEP=':' ;;
esac

# 数据目录 (优先级: 命令行参数 > 环境变量 > 默认值)
DATA_DIR="${1:-${VOIDMEI_FMDATA_DIR:-$HOME/Downloads/voidmei/data/aces/gamedata/flightmodels}}"

echo -e "${CYAN}=== VoidMei FM 数据全量验证 ===${NC}"
echo "项目目录: $PROJECT_ROOT"
echo "数据目录: $DATA_DIR"
echo ""

# 阶段 1: 编译项目源码
echo -e "${YELLOW}[1/3] 编译项目源码...${NC}"
mkdir -p bin
find src -name "*.java" > sources.txt
if ! javac -encoding UTF-8 -d bin -classpath 'dep/*' @sources.txt 2>compile_warnings.txt; then
    echo -e "${RED}编译失败!${NC}"
    cat compile_warnings.txt
    rm -f sources.txt compile_warnings.txt
    exit 2
fi
# 静默处理 deprecation/unchecked 警告
rm -f sources.txt compile_warnings.txt
echo -e "${GREEN}  源码编译完成${NC}"

# 阶段 2: 编译验证器
echo -e "${YELLOW}[2/3] 编译验证器...${NC}"
VALIDATOR_SRC="script/validate/FMDataValidator.java"
if ! javac -encoding UTF-8 -d bin -classpath "dep/*${CPSEP}bin" "$VALIDATOR_SRC" 2>&1; then
    echo -e "${RED}验证器编译失败!${NC}"
    exit 2
fi
echo -e "${GREEN}  验证器编译完成${NC}"

# 阶段 3: 运行验证
echo -e "${YELLOW}[3/3] 运行 FM 数据全量验证...${NC}"
echo ""

if java -classpath "dep/*${CPSEP}bin" FMDataValidator "$DATA_DIR"; then
    echo ""
    echo -e "${GREEN}=== 验证通过 ✅ ===${NC}"
    exit 0
else
    EXIT_CODE=$?
    echo ""
    echo -e "${RED}=== 验证失败 ❌ (退出码: $EXIT_CODE) ===${NC}"
    exit $EXIT_CODE
fi
