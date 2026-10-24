import android.app.UiAutomation;
import android.graphics.Rect;
import android.view.accessibility.AccessibilityNodeInfo;

public final class AccessibilityProbe {
    static final String WRAPPER = "com.android.uiautomator.core.UiAutomationShellWrapper";

    static String attribute(String name, Object value) {
        return " " + name + "=\"" + escape(value) + "\"";
    }

    static String escape(Object value) {
        return value == null ? "" : value.toString().replace("&", "&amp;").replace("\"", "&quot;")
            .replace("<", "&lt;").replace(">", "&gt;").replace("\n", " ");
    }

    static void write(StringBuilder out, AccessibilityNodeInfo node, int index) {
        Rect bounds = new Rect();
        node.getBoundsInScreen(bounds);
        out.append("<node").append(attribute("index", index))
            .append(attribute("text", node.getText())).append(attribute("resource-id", node.getViewIdResourceName()))
            .append(attribute("class", node.getClassName())).append(attribute("package", node.getPackageName()))
            .append(attribute("content-desc", node.getContentDescription())).append(attribute("hint", node.getHintText()))
            .append(attribute("checkable", node.isCheckable())).append(attribute("checked", node.isChecked()))
            .append(attribute("clickable", node.isClickable())).append(attribute("enabled", node.isEnabled()))
            .append(attribute("focusable", node.isFocusable())).append(attribute("focused", node.isFocused()))
            .append(attribute("scrollable", node.isScrollable())).append(attribute("long-clickable", node.isLongClickable()))
            .append(attribute("password", node.isPassword())).append(attribute("selected", node.isSelected()))
            .append(attribute("heading", node.isHeading())).append(attribute("editable", node.isEditable()))
            .append(attribute("bounds", "[" + bounds.left + "," + bounds.top + "][" + bounds.right + "," + bounds.bottom + "]")).append(">");
        for (int child = 0; child < node.getChildCount(); child++) {
            AccessibilityNodeInfo item = node.getChild(child);
            if (item != null) write(out, item, child);
        }
        out.append("</node>");
    }

    public static void main(String[] args) throws Exception {
        Class<?> type = Class.forName(WRAPPER);
        Object wrapper = type.getConstructor().newInstance();
        type.getMethod("connect").invoke(wrapper);
        try {
            type.getMethod("setCompressedLayoutHierarchy", boolean.class).invoke(wrapper, true);
            UiAutomation automation = (UiAutomation) type.getMethod("getUiAutomation").invoke(wrapper);
            automation.waitForIdle(1000, 10000);
            AccessibilityNodeInfo root = automation.getRootInActiveWindow();
            StringBuilder out = new StringBuilder("<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><hierarchy rotation=\"0\">");
            if (root != null) write(out, root, 0);
            System.out.println(out.append("</hierarchy>"));
        } finally {
            type.getMethod("disconnect").invoke(wrapper);
        }
    }
}
