javac -encoding UTF-8 -d bin -classpath dep/* src/prog/* src/parser/* src/ui/*
jar -cvfm VoidMei.jar MANIFEST.MF -C ./bin .
launch4jc ./script/voidmeil4j.xml