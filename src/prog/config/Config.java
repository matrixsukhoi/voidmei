package prog.config;

import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.util.Properties;
import prog.Application;

/**
 * Properties file reader/writer utility.
 * 
 * @author Qutr
 */
public class Config {
    private Properties propertie;
    private InputStreamReader inputFile;
    private FileOutputStream outputFile;

    public Config() {
        propertie = new Properties();
    }

    public Config(String filePath) {
        propertie = new Properties();
        try {
            InputStream in = new FileInputStream(filePath);
            inputFile = new InputStreamReader(in, "utf-8");
            propertie.load(inputFile);
            inputFile.close();
        } catch (FileNotFoundException ex) {
            Application.debugPrint("读取属性文件--->失败！- 原因：文件路径错误或者文件不存在");
            ex.printStackTrace();
        } catch (IOException ex) {
            Application.debugPrint("装载文件--->失败!");
            ex.printStackTrace();
        }
    }

    public String getValue(String key) {
        if (propertie.containsKey(key)) {
            String value = propertie.getProperty(key);
            return value;
        } else
            return "";
    }

    public String getValue(String fileName, String key) {
        try {
            String value = "";
            InputStream in = new FileInputStream(fileName);
            inputFile = new InputStreamReader(in, "utf-8");
            propertie.load(inputFile);
            inputFile.close();
            if (propertie.containsKey(key)) {
                value = propertie.getProperty(key);
                return value;
            } else
                return value;
        } catch (FileNotFoundException e) {
            e.printStackTrace();
            return "";
        } catch (IOException e) {
            e.printStackTrace();
            return "";
        } catch (Exception ex) {
            ex.printStackTrace();
            return "";
        }
    }

    public void clear() {
        propertie.clear();
    }

    public void setValue(String key, String value) {
        propertie.setProperty(key, value);
    }

    public void saveFile(String fileName, String description) {
        try {
            outputFile = new FileOutputStream(fileName);
            propertie.store(outputFile, description);
            outputFile.close();
        } catch (FileNotFoundException e) {
            e.printStackTrace();
        } catch (IOException ioe) {
            ioe.printStackTrace();
        }
    }
}
