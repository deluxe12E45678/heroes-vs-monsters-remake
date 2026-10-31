package com.benny.hvsm;

import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;

/** NativeActivity with sticky immersive fullscreen (status + navigation bar hidden) drawing into the display cutout. */
public class MainActivity extends android.app.NativeActivity {
    @Override protected void onCreate(Bundle b) {
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams lp = getWindow().getAttributes();
            lp.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(lp);
        }
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN | WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        super.onCreate(b);
        hide();
    }
    @Override public void onWindowFocusChanged(boolean f) { super.onWindowFocusChanged(f); if (f) hide(); }
    @Override protected void onResume() { super.onResume(); hide(); }
    private void hide() {
        getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY | View.SYSTEM_UI_FLAG_FULLSCREEN
            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN);
    }
}
