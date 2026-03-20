#!/bin/bash
# 确保JDK 1.8和launch4j环境目录已配置
# 编译
mkdir -p bin
# 找到 src 下所有以 .java 结尾的文件
find src -name "*.java" > sources.txt
# 使用 @ 符号让 javac 读取文件列表
javac -encoding UTF-8 -d bin -classpath 'dep/*' @sources.txt
# 打包
jar -cvfm VoidMei.jar MANIFEST.MF -C ./bin .
# launch4j打包为exe
launch4j ./script/voidmeil4j.xml

pushd ~/Downloads/voidmei
./voidmei.sh
popd
