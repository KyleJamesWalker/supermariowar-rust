package com.kylejameswalker.supermariowar;

import android.content.pm.ApplicationInfo;
import android.content.res.AssetManager;
import android.graphics.Rect;
import android.os.Build;
import android.os.Bundle;
import android.util.Log;
import android.view.DisplayCutout;
import android.view.View;
import android.view.WindowInsets;

import org.libsdl.app.SDLActivity;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;

public class MainActivity extends SDLActivity {
    private static final String TAG = "smw";

    @Override
    protected String[] getLibraries() {
        return new String[] {
            "SDL2",
            "SDL2_image",
            "SDL2_mixer",
            "main"
        };
    }

    private static native void nativeSetCutouts(int l1, int t1, int r1, int b1, int l2, int t2, int r2, int b2);

    private DisplayCutout cutout;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (Build.VERSION.SDK_INT < 28 || mSurface == null) {
            return;
        }
        // The game draws its touch controls beside the picture, where a display cutout may be.
        getWindow().getDecorView().setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
            @Override
            public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
                cutout = insets.getDisplayCutout();
                reportCutouts();
                return view.onApplyWindowInsets(insets);
            }
        });
        mSurface.addOnLayoutChangeListener(new View.OnLayoutChangeListener() {
            @Override
            public void onLayoutChange(View v, int l, int t, int r, int b, int ol, int ot, int or, int ob) {
                reportCutouts();
            }
        });
    }

    /** Up to two cutouts' bounding boxes, relative to the surface the game draws on. */
    private void reportCutouts() {
        int[] at = new int[2];
        int[] origin = new int[2];
        mSurface.getLocationOnScreen(at);
        getWindow().getDecorView().getLocationOnScreen(origin);
        int[] box = new int[8];
        if (cutout != null) {
            List<Rect> rects = cutout.getBoundingRects();
            for (int i = 0; i < Math.min(2, rects.size()); i++) {
                Rect r = rects.get(i);
                box[4 * i] = r.left - (at[0] - origin[0]);
                box[4 * i + 1] = r.top - (at[1] - origin[1]);
                box[4 * i + 2] = r.right - (at[0] - origin[0]);
                box[4 * i + 3] = r.bottom - (at[1] - origin[1]);
            }
        }
        try {
            nativeSetCutouts(box[0], box[1], box[2], box[3], box[4], box[5], box[6], box[7]);
        } catch (UnsatisfiedLinkError e) {
            Log.e(TAG, "libmain is not loaded", e);
        }
    }

    @Override
    protected String[] getArguments() {
        extractData();

        Bundle extras = getIntent().getExtras();
        boolean debuggable = (getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0;
        if (extras == null || !debuggable) {
            return new String[0];
        }
        // Lets tests drive the replay harness: am start --es SMW_REPLAY <file> --esa args <a>,<b>
        for (String key : extras.keySet()) {
            if (key.startsWith("SMW_")) {
                nativeSetenv(key, String.valueOf(extras.get(key)));
            }
        }
        String[] args = extras.getStringArray("args");
        return args != null ? args : new String[0];
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        // The game keeps its state in statics, so a relaunch must start a fresh process.
        if (isFinishing()) {
            android.os.Process.killProcess(android.os.Process.myPid());
        }
    }

    /** The game lists and reads data/ on the filesystem, so copy it out of the APK once per install. */
    private void extractData() {
        File root = getExternalFilesDir(null);
        if (root == null) {
            root = getFilesDir();
        }
        File data = new File(root, "data");
        File stamp = new File(data, ".apk-stamp");
        String version;
        try {
            version = String.valueOf(getPackageManager().getPackageInfo(getPackageName(), 0).lastUpdateTime);
        } catch (Exception e) {
            version = "unknown";
        }
        try {
            if (stamp.exists() && version.equals(readFile(stamp))) {
                return;
            }
            long start = System.currentTimeMillis();
            copyAssets(getAssets(), "data", data);
            try (OutputStream out = new FileOutputStream(stamp)) {
                out.write(version.getBytes(StandardCharsets.UTF_8));
            }
            Log.i(TAG, "Extracted data to " + data + " in " + (System.currentTimeMillis() - start) + " ms");
        } catch (IOException e) {
            Log.e(TAG, "Could not extract data to " + data, e);
        }
    }

    private static String readFile(File file) throws IOException {
        try (InputStream in = new FileInputStream(file)) {
            return new String(readAll(in), StandardCharsets.UTF_8);
        }
    }

    private static byte[] readAll(InputStream in) throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        copy(in, out);
        return out.toByteArray();
    }

    private static void copy(InputStream in, OutputStream out) throws IOException {
        byte[] buffer = new byte[65536];
        int n;
        while ((n = in.read(buffer)) > 0) {
            out.write(buffer, 0, n);
        }
    }

    private static void copyAssets(AssetManager assets, String path, File target) throws IOException {
        String[] children = assets.list(path);
        if (children != null && children.length > 0) {
            target.mkdirs();
            for (String child : children) {
                copyAssets(assets, path + "/" + child, new File(target, child));
            }
            return;
        }
        try (InputStream in = assets.open(path); OutputStream out = new FileOutputStream(target)) {
            copy(in, out);
        }
    }
}
