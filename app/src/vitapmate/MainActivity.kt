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
        }
        // The splash leaves as soon as the first frame is drawn (nothing holds it): a quick fade
        // with the icon growing a little, even mid-animation.
        if (Build.VERSION.SDK_INT >= 31) {
            splashScreen.setOnExitAnimationListener { view ->
                view.iconView?.animate()?.scaleX(1.15f)?.scaleY(1.15f)?.setDuration(180)?.start()
                view.animate().alpha(0f).setDuration(180).withEndAction(view::remove).start()
            }
        }
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
                CompositionLocalProvider(LocalOpenRequest provides openRequest) { App() }
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
    }
}
