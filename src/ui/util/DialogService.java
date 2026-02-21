package ui.util;

import prog.AlwaysOnTopCoordinator;
import com.alee.laf.optionpane.WebOptionPane;
import java.awt.Component;

/**
 * Centralized dialog service that coordinates with AlwaysOnTopCoordinator
 * to ensure dialogs are not blocked by always-on-top overlays.
 *
 * <p>This service works even when dialogs are triggered before overlays exist,
 * because AlwaysOnTopCoordinator tracks pending dialog state independently.
 *
 * Usage: Replace direct WebOptionPane calls with DialogService calls.
 * Example:
 *   // Before: WebOptionPane.showConfirmDialog(parent, msg, title, YES_NO_OPTION, WARNING_MESSAGE);
 *   // After:  DialogService.showConfirmDialog(parent, msg, title, YES_NO_OPTION, WARNING_MESSAGE);
 */
public class DialogService {

    /**
     * Show a confirmation dialog, temporarily lowering overlay z-order.
     *
     * @param parent the parent component for the dialog
     * @param message the message to display
     * @param title the dialog title
     * @param optionType the option type (e.g., YES_NO_OPTION)
     * @param messageType the message type (e.g., WARNING_MESSAGE)
     * @return the option chosen by the user
     */
    public static int showConfirmDialog(Component parent, Object message,
            String title, int optionType, int messageType) {
        AlwaysOnTopCoordinator.getInstance().dialogWillShow();
        try {
            return WebOptionPane.showConfirmDialog(parent, message, title, optionType, messageType);
        } finally {
            AlwaysOnTopCoordinator.getInstance().dialogDidDismiss();
        }
    }

    /**
     * Show a message dialog, temporarily lowering overlay z-order.
     *
     * @param parent the parent component for the dialog
     * @param message the message to display
     * @param title the dialog title
     * @param messageType the message type (e.g., ERROR_MESSAGE, INFORMATION_MESSAGE)
     */
    public static void showMessageDialog(Component parent, Object message,
            String title, int messageType) {
        AlwaysOnTopCoordinator.getInstance().dialogWillShow();
        try {
            WebOptionPane.showMessageDialog(parent, message, title, messageType);
        } finally {
            AlwaysOnTopCoordinator.getInstance().dialogDidDismiss();
        }
    }
}
