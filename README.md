# VoidMei - 战争雷霆8111端口Java图形前端

# 工作原理
- 通过HTTP/GET请求读取127.0.01:8111端口中的飞行状态(state)以及飞行仪表(indicators)信息
- 解析离线拆包的气动模型文件(FM blkx)
- 综合以上信息,以图形界面的形式呈现给用户

# 编译
- 使用eclipse导入工程,程序入口设置到app.java中的main函数
- 导入外部UI库 weblaf-complete-1.29.jar

# 代码结构说明
代码结构,变量命名目前还比较乱,后期有时间会调整
- src/prog/app.java - 程序入口
- src/prog/controller.java - 程序状态转换控制
- src/prog/service.java - 主HTTP数据请求与处理线程
- src/prog/uiThread.java - UI绘制线程
- src/parser - state/indicator/blkx等解析器代码
- src/ui - 各ui界面的绘制代码
