#!/bin/bash
# 获得当前版本号
version=`cat ./src/prog/Application.java | grep "String version" | grep "[0-9]\+.[0-9]\+" -o`
# 把version字符串中的小数点替换成下划线
version=${version//./_}
echo "current version is ${version}"
# 使用tar打包项目
# tar -cjf VoidMei_v${version}.bz2 -T ./script/ziplist.txt
# 使用zip打包项目
zip -r -@ VoidMei_v${version}.zip < ./script/ziplist.txt