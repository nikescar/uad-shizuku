package pe.nikescar.uad_shizuku;

import android.app.Activity;
import android.util.Log;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;

/**
 * Helper class that attaches a ScaleGestureDetector to the Activity's content view.
 * Accumulates the pinch scale factor for consumption by Rust via JNI.
 *
 * Touch events are observed but not consumed (onTouch returns false),
 * so they still flow to the NativeActivity's native input queue.
 */
public class ScaleGestureHelper implements
        ScaleGestureDetector.OnScaleGestureListener,
        View.OnTouchListener {

    private static final String TAG = "ScaleGestureHelper";

    private static ScaleGestureHelper sInstance;
    private ScaleGestureDetector mDetector;

    // Accumulated scale factor since last query from Rust.
    // Protected by synchronized access.
    private final Object mLock = new Object();
    private float mAccumulatedScale = 1.0f;

    /**
     * Initialize the ScaleGestureHelper and attach to the Activity's content view.
     * Must be called on the UI thread.
     */
    public static void init(final Activity activity) {
        if (sInstance != null) {
            return; // already initialized
        }

        activity.runOnUiThread(() -> {
            try {
                sInstance = new ScaleGestureHelper();
                sInstance.mDetector = new ScaleGestureDetector(activity, sInstance);

                // Get the content view (NativeContentView lives inside this)
                View contentView = activity.getWindow().getDecorView();
                // Find the actual NativeActivity surface view
                View nativeView = activity.findViewById(android.R.id.content);
                if (nativeView != null) {
                    nativeView.setOnTouchListener(sInstance);
                    Log.i(TAG, "ScaleGestureDetector attached to content view");
                } else {
                    contentView.setOnTouchListener(sInstance);
                    Log.i(TAG, "ScaleGestureDetector attached to decor view (fallback)");
                }
            } catch (Exception e) {
                Log.e(TAG, "Failed to initialize ScaleGestureHelper", e);
                sInstance = null;
            }
        });
    }

    /**
     * Get the accumulated scale factor since the last call and reset it to 1.0.
     * Called from Rust each frame.
     */
    public static float getAndResetScale() {
        if (sInstance == null) {
            return 1.0f;
        }
        synchronized (sInstance.mLock) {
            float scale = sInstance.mAccumulatedScale;
            sInstance.mAccumulatedScale = 1.0f;
            return scale;
        }
    }

    // --- View.OnTouchListener ---

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        if (mDetector != null) {
            mDetector.onTouchEvent(event);
        }
        // Return false so the event still propagates to the native input queue
        return false;
    }

    // --- ScaleGestureDetector.OnScaleGestureListener ---

    @Override
    public boolean onScale(ScaleGestureDetector detector) {
        float factor = detector.getScaleFactor();
        synchronized (mLock) {
            mAccumulatedScale *= factor;
        }
        return true;
    }

    @Override
    public boolean onScaleBegin(ScaleGestureDetector detector) {
        return true; // Accept the gesture
    }

    @Override
    public void onScaleEnd(ScaleGestureDetector detector) {
        // Nothing to do
    }
}
