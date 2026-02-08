package pe.nikescar.uad_shizuku;

import android.content.ComponentName;
import android.content.ServiceConnection;
import android.content.pm.PackageManager;
import android.os.IBinder;
import android.os.RemoteException;
import android.util.Log;

import java.util.concurrent.atomic.AtomicInteger;

import rikka.shizuku.Shizuku;

public class ShizukuBridge {

    private static final String TAG = "ShizukuBridge";
    private static final String APPLICATION_ID = "pe.nikescar.uad_shizuku";
    private static final int VERSION_CODE = 1;

    private static volatile IShellService sShellService = null;
    private static ServiceConnection sConnection = null;
    private static Shizuku.UserServiceArgs sServiceArgs = null;

    // Permission state: 0=unknown, 1=requesting, 2=granted, 3=denied
    private static final AtomicInteger sPermissionState = new AtomicInteger(0);

    // Bind state: 0=not bound, 1=binding, 2=bound, 3=failed
    private static final AtomicInteger sBindState = new AtomicInteger(0);

    private static final Shizuku.OnRequestPermissionResultListener sPermissionListener =
        (requestCode, grantResult) -> {
            Log.i(TAG, "Permission result: requestCode=" + requestCode +
                  " grantResult=" + grantResult);
            if (grantResult == PackageManager.PERMISSION_GRANTED) {
                sPermissionState.set(2); // granted
            } else {
                sPermissionState.set(3); // denied
            }
        };

    /**
     * Initialize ShizukuBridge: register permission result listener.
     * Call once during app startup.
     */
    public static void init() {
        try {
            Shizuku.addRequestPermissionResultListener(sPermissionListener);
            Log.i(TAG, "Permission listener registered");
        } catch (Exception e) {
            Log.e(TAG, "Failed to register permission listener", e);
        }
    }

    public static boolean isAvailable() {
        try {
            return Shizuku.pingBinder();
        } catch (Exception e) {
            Log.e(TAG, "isAvailable failed", e);
            return false;
        }
    }

    public static boolean hasPermission() {
        try {
            boolean granted = Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED;
            if (granted) {
                sPermissionState.set(2);
            }
            return granted;
        } catch (Exception e) {
            Log.e(TAG, "hasPermission failed", e);
            return false;
        }
    }

    public static void requestPermission() {
        try {
            sPermissionState.set(1); // requesting
            Shizuku.requestPermission(0);
        } catch (Exception e) {
            Log.e(TAG, "requestPermission failed", e);
            sPermissionState.set(3); // error = denied
        }
    }

    /**
     * Returns the permission state.
     * 0=unknown, 1=requesting, 2=granted, 3=denied
     */
    public static int getPermissionState() {
        return sPermissionState.get();
    }

    /**
     * Non-blocking service bind. Starts the bind process and returns immediately.
     * Poll getBindState() to check progress.
     * Returns true only if already bound.
     */
    public static boolean bindService() {
        if (sShellService != null) {
            sBindState.set(2);
            return true;
        }

        if (sBindState.get() == 1) {
            return false; // Already binding
        }

        sBindState.set(1); // binding in progress

        try {
            sServiceArgs = new Shizuku.UserServiceArgs(
                    new ComponentName(APPLICATION_ID, ShellService.class.getName()))
                    .daemon(false)
                    .processNameSuffix("shizuku_shell")
                    .debuggable(true)
                    .version(VERSION_CODE);

            sConnection = new ServiceConnection() {
                @Override
                public void onServiceConnected(ComponentName name, IBinder binder) {
                    if (binder != null && binder.pingBinder()) {
                        sShellService = IShellService.Stub.asInterface(binder);
                        sBindState.set(2); // bound
                        Log.i(TAG, "ShellService connected");
                    } else {
                        sBindState.set(3); // failed
                        Log.e(TAG, "ShellService binder invalid");
                    }
                }

                @Override
                public void onServiceDisconnected(ComponentName name) {
                    sShellService = null;
                    sBindState.set(0); // not bound
                    Log.w(TAG, "ShellService disconnected");
                }
            };

            Shizuku.bindUserService(sServiceArgs, sConnection);
        } catch (Exception e) {
            Log.e(TAG, "bindService failed", e);
            sBindState.set(3); // failed
            return false;
        }

        return false; // Not yet bound, caller should poll getBindState()
    }

    /**
     * Returns the bind state.
     * 0=not bound, 1=binding, 2=bound, 3=failed
     */
    public static int getBindState() {
        return sBindState.get();
    }

    public static String execCommand(String command) {
        if (sShellService == null) {
            return "ERROR: ShellService not bound";
        }
        try {
            return sShellService.execCommand(command);
        } catch (RemoteException e) {
            Log.e(TAG, "execCommand failed", e);
            return "ERROR: " + e.getMessage();
        }
    }

    /**
     * Execute a command and write output to a file (bypasses Binder size limit).
     * Returns null on success, or an error message on failure.
     */
    public static String execCommandToFile(String command, String outputPath) {
        if (sShellService == null) {
            return "ERROR: ShellService not bound";
        }
        try {
            sShellService.execCommandToFile(command, outputPath);
            return null; // success
        } catch (RemoteException e) {
            Log.e(TAG, "execCommandToFile failed", e);
            return "ERROR: " + e.getMessage();
        }
    }

    public static boolean isServiceBound() {
        return sShellService != null;
    }

    public static void unbindService() {
        if (sConnection != null) {
            try {
                Shizuku.unbindUserService(sServiceArgs, sConnection, true);
            } catch (Exception e) {
                Log.e(TAG, "unbindService failed", e);
            }
            sConnection = null;
            sServiceArgs = null;
        }
        sShellService = null;
        sBindState.set(0);
    }

    public static void cleanup() {
        try {
            Shizuku.removeRequestPermissionResultListener(sPermissionListener);
        } catch (Exception e) {
            Log.e(TAG, "Failed to remove permission listener", e);
        }
    }
}
