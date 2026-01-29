package prog.event;

public class GameStatusEvent {

    public enum Status {
        INIT,
        CONNECTED,
        IN_GAME,
        PREVIEW
    }

    private final Status status;

    public GameStatusEvent(Status status) {
        this.status = status;
    }

    public Status getStatus() {
        return status;
    }
}
