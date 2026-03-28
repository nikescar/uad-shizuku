package pe.nikescar.uad_shizuku;

import android.content.IIntentSender;
import android.content.IntentSender;

public class IntentSenderUtils {

    public static IntentSender newInstance(IIntentSender sender) throws Exception {
        return IntentSender.class.getConstructor(IIntentSender.class).newInstance(sender);
    }
}
