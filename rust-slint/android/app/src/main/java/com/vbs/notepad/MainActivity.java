package com.vbs.notepad;

/**
 * The only Java in the app. android.app.NativeActivity loads libnotepad.so
 * (named by the android.app.lib_name meta-data) and hands it the event loop;
 * everything after that point is Rust.
 */
public class MainActivity extends android.app.NativeActivity {
}
