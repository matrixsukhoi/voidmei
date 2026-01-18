package ui.component.row;

import java.awt.Font;

/**
 * Row displaying Flaps/Gear/Airbrake status (Row 2).
 */
public class HUDFlapsRow extends HUDTextRow {

    public HUDFlapsRow(int index, Font font, int height) {
        super(index, font, height);
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;
        this.update(data.flapsStr, data.warnConfiguration);
    }
}
