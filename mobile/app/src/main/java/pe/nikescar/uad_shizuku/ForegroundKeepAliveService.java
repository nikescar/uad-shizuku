package pe.nikescar.uad_shizuku;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.IBinder;

/**
 * Foreground service that keeps the UAD-Shizuku process alive while the
 * NativeActivity is backgrounded (e.g. screen off / device sleep), so the
 * process is not reclaimed by the system and app state survives resume.
 */
public class ForegroundKeepAliveService extends Service {

    private static final String CHANNEL_ID = "uad_shizuku_keep_alive";
    private static final int NOTIFICATION_ID = 1;

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        startForegroundInternal();
        return START_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private void startForegroundInternal() {
        createNotificationChannel();

        Intent launchIntent = getPackageManager().getLaunchIntentForPackage(getPackageName());
        PendingIntent contentIntent = null;
        if (launchIntent != null) {
            int piFlags = PendingIntent.FLAG_UPDATE_CURRENT;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                piFlags |= PendingIntent.FLAG_IMMUTABLE;
            }
            contentIntent = PendingIntent.getActivity(this, 0, launchIntent, piFlags);
        }

        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            builder = new Notification.Builder(this, CHANNEL_ID);
        } else {
            builder = new Notification.Builder(this);
        }
        builder.setContentTitle("UAD-Shizuku")
                .setContentText("Keeping app session alive")
                .setSmallIcon(android.R.drawable.ic_menu_manage)
                .setOngoing(true);
        if (contentIntent != null) {
            builder.setContentIntent(contentIntent);
        }

        Notification notification = builder.build();

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(NOTIFICATION_ID, notification,
                    android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE);
        } else {
            startForeground(NOTIFICATION_ID, notification);
        }
    }

    private void createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationManager manager = getSystemService(NotificationManager.class);
            if (manager != null && manager.getNotificationChannel(CHANNEL_ID) == null) {
                NotificationChannel channel = new NotificationChannel(
                        CHANNEL_ID,
                        "Keep Alive",
                        NotificationManager.IMPORTANCE_MIN);
                channel.setDescription("Keeps UAD-Shizuku running while the device is asleep");
                channel.setShowBadge(false);
                manager.createNotificationChannel(channel);
            }
        }
    }

    /**
     * JNI-callable entry point: request the POST_NOTIFICATIONS runtime permission
     * (API 33+) so the foreground service's notification is actually visible.
     * No-op on older API levels, where the permission does not exist.
     */
    public static void requestNotificationPermission(Activity activity) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.requestPermissions(
                    new String[]{"android.permission.POST_NOTIFICATIONS"}, 1001);
        }
    }

    /**
     * JNI-callable entry point: start the foreground keep-alive service.
     */
    public static void startService(Context context) {
        Intent intent = new Intent(context, ForegroundKeepAliveService.class);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent);
        } else {
            context.startService(intent);
        }
    }

    /**
     * JNI-callable entry point: stop the foreground keep-alive service.
     */
    public static void stopService(Context context) {
        Intent intent = new Intent(context, ForegroundKeepAliveService.class);
        context.stopService(intent);
    }
}
