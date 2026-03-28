package pe.nikescar.uad_shizuku;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentSender;
import android.content.ServiceConnection;
import android.content.pm.IPackageInstaller;
import android.content.pm.IPackageInstallerSession;
import android.content.pm.PackageInstaller;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.IBinder;
import android.os.Process;
import android.os.RemoteException;
import android.util.Log;

import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicInteger;

import rikka.shizuku.Shizuku;
import rikka.shizuku.ShizukuBinderWrapper;
import rikka.shizuku.SystemServiceHelper;

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

    /**
     * Install an APK file using Shizuku PackageInstaller API.
     * @param apkPath Full path to the APK file to install
     * @param context Application context for API < 26
     * @return Status code: PackageInstaller.STATUS_SUCCESS (0) on success, other codes on failure
     */
    public static int installApk(String apkPath, Context context) {
        PackageInstaller packageInstaller = null;
        PackageInstaller.Session session = null;

        try {
            // Get IPackageInstaller from system service
            IBinder packageManagerBinder = SystemServiceHelper.getSystemService("package");
            if (packageManagerBinder == null) {
                Log.e(TAG, "Failed to get package manager service");
                return PackageInstaller.STATUS_FAILURE;
            }

            android.content.pm.IPackageManager iPackageManager =
                android.content.pm.IPackageManager.Stub.asInterface(
                    new ShizukuBinderWrapper(packageManagerBinder));

            IPackageInstaller iPackageInstaller = iPackageManager.getPackageInstaller();

            // Determine installer package name and user ID
            boolean isRoot = Shizuku.getUid() == 0;
            String installerPackageName = isRoot ? APPLICATION_ID : "com.android.shell";
            int userId = isRoot ? Process.myUserHandle().hashCode() : 0;

            // Get attribution tag (null for API < 31)
            String attributionTag = null;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                attributionTag = context.getAttributionTag();
            }

            // Create PackageInstaller instance using reflection
            packageInstaller = createPackageInstaller(iPackageInstaller, installerPackageName,
                                                     attributionTag, userId, context);

            // Create session params
            PackageInstaller.SessionParams params = new PackageInstaller.SessionParams(
                PackageInstaller.SessionParams.MODE_FULL_INSTALL);

            // Set install flags using reflection
            int installFlags = getInstallFlags(params);
            installFlags |= 0x00000004 /*INSTALL_ALLOW_TEST*/ | 0x00000002 /*INSTALL_REPLACE_EXISTING*/;
            setInstallFlags(params, installFlags);

            // Create session
            int sessionId = packageInstaller.createSession(params);
            Log.i(TAG, "Created install session: " + sessionId);

            // Open session
            IPackageInstallerSession iSession = IPackageInstallerSession.Stub.asInterface(
                new ShizukuBinderWrapper(iPackageInstaller.openSession(sessionId).asBinder()));
            session = createSession(iSession);

            // Write APK to session
            try (InputStream is = new FileInputStream(apkPath);
                 OutputStream os = session.openWrite("base.apk", 0, -1)) {

                byte[] buffer = new byte[8192];
                int len;
                while ((len = is.read(buffer)) > 0) {
                    os.write(buffer, 0, len);
                    os.flush();
                    session.fsync(os);
                }
            }

            Log.i(TAG, "APK written to session");

            // Commit session and wait for result
            final Intent[] results = new Intent[]{null};
            final CountDownLatch latch = new CountDownLatch(1);

            IntentSender intentSender = IntentSenderUtils.newInstance(new IIntentSenderAdaptor() {
                @Override
                public void send(Intent intent) {
                    results[0] = intent;
                    latch.countDown();
                }
            });

            session.commit(intentSender);

            // Wait for result (with timeout)
            latch.await();

            Intent result = results[0];
            if (result == null) {
                Log.e(TAG, "Install result is null");
                return PackageInstaller.STATUS_FAILURE;
            }

            int status = result.getIntExtra(PackageInstaller.EXTRA_STATUS,
                                           PackageInstaller.STATUS_FAILURE);
            String message = result.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);

            Log.i(TAG, "Install status: " + status + " (" + message + ")");
            return status;

        } catch (Exception e) {
            Log.e(TAG, "installApk failed", e);
            return PackageInstaller.STATUS_FAILURE;
        } finally {
            if (session != null) {
                try {
                    session.close();
                } catch (Exception e) {
                    Log.e(TAG, "Failed to close session", e);
                }
            }
        }
    }

    /**
     * Uninstall a package using Shizuku PackageInstaller API.
     * @param packageName Package name to uninstall
     * @param context Application context for API < 26
     * @return Status code: PackageInstaller.STATUS_SUCCESS (0) on success, other codes on failure
     */
    public static int uninstallPackage(String packageName, Context context) {
        try {
            // Get IPackageInstaller from system service
            IBinder packageManagerBinder = SystemServiceHelper.getSystemService("package");
            if (packageManagerBinder == null) {
                Log.e(TAG, "Failed to get package manager service");
                return PackageInstaller.STATUS_FAILURE;
            }

            android.content.pm.IPackageManager iPackageManager =
                android.content.pm.IPackageManager.Stub.asInterface(
                    new ShizukuBinderWrapper(packageManagerBinder));

            IPackageInstaller iPackageInstaller = iPackageManager.getPackageInstaller();

            // Determine installer package name and user ID
            boolean isRoot = Shizuku.getUid() == 0;
            String installerPackageName = isRoot ? APPLICATION_ID : "com.android.shell";
            int userId = isRoot ? Process.myUserHandle().hashCode() : 0;

            // Get attribution tag (null for API < 31)
            String attributionTag = null;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                attributionTag = context.getAttributionTag();
            }

            // Create PackageInstaller instance
            PackageInstaller packageInstaller = createPackageInstaller(iPackageInstaller,
                                                                       installerPackageName,
                                                                       attributionTag,
                                                                       userId, context);

            // Uninstall package and wait for result
            final Intent[] results = new Intent[]{null};
            final CountDownLatch latch = new CountDownLatch(1);

            IntentSender intentSender = IntentSenderUtils.newInstance(new IIntentSenderAdaptor() {
                @Override
                public void send(Intent intent) {
                    results[0] = intent;
                    latch.countDown();
                }
            });

            packageInstaller.uninstall(packageName, intentSender);

            // Wait for result
            latch.await();

            Intent result = results[0];
            if (result == null) {
                Log.e(TAG, "Uninstall result is null");
                return PackageInstaller.STATUS_FAILURE;
            }

            int status = result.getIntExtra(PackageInstaller.EXTRA_STATUS,
                                           PackageInstaller.STATUS_FAILURE);
            String message = result.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);

            Log.i(TAG, "Uninstall status: " + status + " (" + message + ")");
            return status;

        } catch (Exception e) {
            Log.e(TAG, "uninstallPackage failed", e);
            return PackageInstaller.STATUS_FAILURE;
        }
    }

    // Helper methods using reflection to access hidden APIs

    private static PackageInstaller createPackageInstaller(IPackageInstaller installer,
                                                           String installerPackageName,
                                                           String installerAttributionTag,
                                                           int userId,
                                                           Context context)
            throws Exception {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            return PackageInstaller.class.getConstructor(
                IPackageInstaller.class, String.class, String.class, int.class)
                .newInstance(installer, installerPackageName, installerAttributionTag, userId);
        } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            return PackageInstaller.class.getConstructor(
                IPackageInstaller.class, String.class, int.class)
                .newInstance(installer, installerPackageName, userId);
        } else {
            return PackageInstaller.class.getConstructor(
                Context.class, PackageManager.class, IPackageInstaller.class, String.class, int.class)
                .newInstance(context, context.getPackageManager(), installer, installerPackageName, userId);
        }
    }

    private static PackageInstaller.Session createSession(IPackageInstallerSession session)
            throws Exception {
        return PackageInstaller.Session.class.getConstructor(IPackageInstallerSession.class)
            .newInstance(session);
    }

    private static int getInstallFlags(PackageInstaller.SessionParams params) throws Exception {
        return (int) PackageInstaller.SessionParams.class.getDeclaredField("installFlags").get(params);
    }

    private static void setInstallFlags(PackageInstaller.SessionParams params, int newValue)
            throws Exception {
        PackageInstaller.SessionParams.class.getDeclaredField("installFlags").set(params, newValue);
    }
}
