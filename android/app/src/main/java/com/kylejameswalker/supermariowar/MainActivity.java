package com.kylejameswalker.supermariowar;

import android.content.pm.ApplicationInfo;
import android.content.res.AssetManager;
import android.os.Bundle;
import android.util.Log;

import org.libsdl.app.SDLActivity;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

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
