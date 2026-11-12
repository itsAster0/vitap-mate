package vitapmate

import android.graphics.Color
import android.os.Build
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import vitapmate.data.AppCore
import vitapmate.data.Foreground
import vitapmate.data.Settings
import vitapmate.data.ThemeMode
import vitapmate.ui.Palette
import vitapmate.ui.AppTheme
import kotlin.time.Duration.Companion.microseconds
import kotlin.time.Duration.Companion.milliseconds

class MainActivity : ComponentActivity() {
    private val openRequest = OpenRequest()

    override fun onCreate(savedInstanceState: Bundle?) {
        // Settings first: the core reads the developer settings when it starts.
        Settings.load(applicationContext)
        AppCore.start(applicationContext)
        super.onCreate(savedInstanceState)
        opened(intent)
        // Keeps the background job in step with the settings (an update can drop it).
        BackgroundSync.apply(applicationContext)
        if (savedInstanceState == null) {
            BackgroundSync.registerDebugTrigger(applicationContext)
            Push.start(applicationContext)
            PlayUpdate.check(this)
        }
        // The splash leaves as soon as the first frame is drawn (nothing holds it): a quick fade
        // with the icon growing a little, even mid-animation.
        // An icon picked before the splash followed it.
        if (savedInstanceState == null) AppIcon.syncSplash(this)
        if (Build.VERSION.SDK_INT >= 31) {
            splashScreen.setOnExitAnimationListener { view ->
                view.iconView?.animate()?.scaleX(1.15f)?.scaleY(1.15f)?.setDuration(180)?.start()
                view.animate().alpha(0f).setDuration(180).withEndAction(view::remove).start()
            }
        }
        // Before the first frame (Android 15 is edge-to-edge anyway); the bars' icons follow the theme below.
        SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT).let { enableEdgeToEdge(it, it) }
        setContent {
            val dark = when (Settings.themeMode) {
                ThemeMode.System -> isSystemInDarkTheme()
                ThemeMode.Light -> false
                ThemeMode.Dark -> true
            }
            // Transparent bars whose icons follow the theme (the activity handles uiMode itself).
            DisposableEffect(dark) {
                val style = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT) { dark }
                enableEdgeToEdge(style, style)
                onDispose {}
            }
            val choice = Settings.colorChoice
            AppTheme(dark, if (choice == Settings.WALLPAPER) null else Palette.entries.firstOrNull { it.name == choice } ?: Palette.Default, Settings.pureBlack) {
                // Every phone lays out as a 412 dp wide one, within 15% of its own display size;
                // the font size stays the user's, and tablets keep their own density.
                val config = LocalConfiguration.current
                val own = LocalDensity.current
                // Settings → Appearance → Display size scales that, or (System) leaves the phone's own.
                val fitted = if (config.smallestScreenWidthDp >= 600) own.density else (own.density * config.smallestScreenWidthDp / 412f).coerceIn(own.density * 0.85f, own.density * 1.15f)
                val density = if (Settings.displaySize == 0) own else Density(fitted * Settings.displaySize / 100f, own.fontScale)
                CompositionLocalProvider(LocalOpenRequest provides openRequest, LocalDensity provides density) { App() }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        opened(intent)
    }

    /** A change notification ([OPEN_INBOX]) or a launcher shortcut (a feature id) was tapped: the shell opens it. */
    private fun opened(intent: Intent?) {
        intent?.getStringExtra(OPEN)?.let { openRequest.screen = it }
    }

    companion object {
        const val OPEN = "open"
        const val OPEN_INBOX = "inbox"
    }

    override fun onStart() {
        super.onStart()
        Foreground.started()
    }

    override fun onStop() {
        super.onStop()
        Foreground.stopped()
        // Picks up a timetable refreshed in the app.
        lifecycleScope.launch { TimetableWidget.refresh(applicationContext) }
        // And outing pages fetched in it; the class update follows a changed timetable.
        lifecycleScope.launch { LiveUpdates.refresh(applicationContext, "live.leave"); LiveUpdates.outings(applicationContext, "live.leave") }
        lifecycleScope.launch { ClassMode.refresh(applicationContext, "classmode.leave") }
    }
}
