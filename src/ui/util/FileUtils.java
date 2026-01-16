package ui.util;

public class FileUtils {

    public static String[] getFilelistNameNoEx(String[] list) {
        if (list == null)
            return new String[0];
        String[] a = new String[list.length];
        for (int i = 0; i < list.length; i++) {
            a[i] = getFileNameNoEx(list[i]);
        }
        return a;
    }

    public static String getFileNameNoEx(String filename) {
        if ((filename != null) && (filename.length() > 0)) {
            int dot = filename.lastIndexOf('.');
            if ((dot > -1) && (dot < (filename.length()))) {
                return filename.substring(0, dot);
            }
        }
        return filename;
    }
}
