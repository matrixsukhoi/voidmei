package prog.service;

import parser.AttributePool;
import parser.Blkx;
import prog.config.ConfigProvider;
import prog.util.HttpHelper;

public class FlightModelService {

    private final ConfigProvider configProvider;

    // Core Data State
    private Blkx Blkx;
    private String loadedFMName = null;
    private String identifiedFMName = null;
    private long lastBlkxCheckTime = 0;
    private static final long BLKX_CHECK_INTERVAL = 5000;

    public AttributePool globalPool = new AttributePool();

    public FlightModelService(ConfigProvider configProvider) {
        this.configProvider = configProvider;
    }

    /**
     * Just-In-Time getter for Flight Model data.
     * Triggers parsing only if target aircraft differs from loaded one.
     */
    public synchronized Blkx getBlkx() {
        // If we don't even have an identified plane yet, try to find one
        if (identifiedFMName == null) {
            ensureBlkxLoaded();
        }

        if (identifiedFMName == null)
            return null;

        // Skip if already loaded and valid
        if (identifiedFMName.equalsIgnoreCase(loadedFMName) && Blkx != null && Blkx.valid) {
            return Blkx;
        }

        // Trigger actual parsing
        loadFMData(identifiedFMName);
        return Blkx;
    }

    /**
     * IDENTIFY the current aircraft from live source or config.
     * Does NOT trigger parsing unless specifically requested via getBlkx().
     */
    public void ensureBlkxLoaded() {
        // Throttled live polling
        long now = System.currentTimeMillis();
        if (now - lastBlkxCheckTime < BLKX_CHECK_INTERVAL) {
            return;
        }
        lastBlkxCheckTime = now;

        HttpHelper httpDataFetcher = new HttpHelper();
        String livePlaneName = httpDataFetcher.getLiveAircraftType();

        if (livePlaneName != null) {
            identifiedFMName = livePlaneName;
            return;
        }

        // Fallback to config
        String configPlane = configProvider.getConfig("selectedFM0");
        if (configPlane != null && !configPlane.isEmpty()) {
            identifiedFMName = configPlane;
        }
    }

    public synchronized void loadFMData(String planename) {
        if (planename == null || planename.isEmpty())
            return;

        // Skip if truly already loaded (double check for safety)
        if (planename.equalsIgnoreCase(loadedFMName) && Blkx != null && Blkx.valid) {
            return;
        }

        prog.util.Logger.info("FlightModelService", "Lazily Loading Flight Model for: " + planename);

        String fmfile = null;
        String planeFileName = planename.toLowerCase();

        // 1. Try to find the pointer file (.Blkx) in the parent directory
        Blkx lookupBlkx = new Blkx("./data/aces/gamedata/flightmodels/" + planeFileName + ".Blkx",
                planeFileName + ".blk",
                false);
        if (lookupBlkx.valid == true) {
            fmfile = lookupBlkx.getlastone("fmfile");
            if (fmfile != null) {
                fmfile = fmfile.substring(fmfile.indexOf("\"") + 1, fmfile.length() - 1);
                if (fmfile.charAt(0) == '/')
                    fmfile = fmfile.substring(1);
            }
        }

        // 2. Fallback to direct path construction
        if (fmfile == null) {
            fmfile = "fm/" + planeFileName + ".blk";
        }

        if (-1 == fmfile.indexOf(".blk")) {
            fmfile += ".blk";
        }

        // 3. Final parse in the fm/ subdirectory (or wherever the pointer pointed)
        // Note: The original code logic for pointer resolution results in a path
        // relative to flightmodels
        // e.g if fmfile is "fm/bf109.blk", final path is
        // "./data/aces/gamedata/flightmodels/fm/bf109.blkx" ?
        // Wait, original code was: "./data/aces/gamedata/flightmodels/" + fmfile + "x"
        // If fmfile is "fm/foo.blk", then path is ".../flightmodels/fm/foo.blkx"
        // This seems to assume the data files are .blkx but referred to as .blk inside
        // the index?
        // Let's trust the original logic copy-paste for now.

        Blkx = new Blkx("./data/aces/gamedata/flightmodels/" + fmfile + "x", fmfile);

        if (Blkx.valid == true) {
            Blkx.getAllplotdata();
            Blkx.finalizeLoading();
            // Explicitly trigger GC after loading the massive FM data structures
            System.gc();

            // Populate Global Pool
            globalPool.clear(); // Important: Clear old data before adding new
            globalPool.putAll(Blkx.getVariableMap());
            loadedFMName = planename;

            // Notify observers that FM data is ready
            prog.event.UIStateBus.getInstance().publish(prog.event.UIStateEvents.FM_DATA_LOADED, planename);
        }
    }

    public String getIdentifiedFMName() {
        return identifiedFMName;
    }

    public String getLoadedFMName() {
        return loadedFMName;
    }

    public AttributePool getGlobalPool() {
        return globalPool;
    }
}
