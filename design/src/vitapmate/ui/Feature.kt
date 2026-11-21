package vitapmate.ui

import android.content.Context
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.vector.ImageVector
import java.io.File

/**
 * A screen the app can show: in the nav bar, in More, as a route. Each feature module exposes
 * one (`val <name>Feature`); the app lists them in its registry.
 */
@Immutable
data class Feature(
    /** Stable: settings keys, routes, log src `ui.<id>`. */
    val id: String,
    val label: String,
    val icon: ImageVector,
    /** Top app bar actions next to the title. */
    val actions: @Composable RowScope.() -> Unit = {},
    /**
     * A full screen for one item of this feature (a course, a day), opened with
     * [LocalOpenDetail]; the app draws it under a back arrow and the item's title. [item] is the
     * feature's own key; the screen survives process death, so it reloads by that key.
     */
    val detail: (@Composable (item: String) -> Unit)? = null,
    /**
     * [content] is the whole screen: its own bar and insets, no nav bar, no tab bar, never a
     * tab (the VTOP web view, a full route in the Flutter app).
     */
    val fullScreen: Boolean = false,
    /**
     * Makes the page a PDF from what the app has saved (off the main thread; null when there is
     * nothing yet). A long press on the account button then offers to share or print it ([PdfMenu]).
     */
    val pdf: (suspend (context: Context, flow: String) -> File?)? = null,
    val content: @Composable () -> Unit,
)


/** Opens a feature's [Feature.detail] for `item`, titled `title` (the current feature by default, or another by id). */
class DetailOpener(
    private val current: String = "",
    private val push: (feature: String, item: String, title: String) -> Unit = { _, _, _ -> },
) {
    fun open(item: String, title: String) {
        if (current.isNotEmpty()) push(current, item, title)
    }

    fun open(feature: String, item: String, title: String) = push(feature, item, title)
}

/** Provided by the app for each feature's screens and details. */
val LocalOpenDetail = staticCompositionLocalOf { DetailOpener() }

/**
 * Space a feature's scrolling content leaves at the bottom so its last item clears the floating
 * nav bar and the gesture area. Lists use it as their bottom content padding.
 */
val LocalContentPadding = staticCompositionLocalOf { PaddingValues() }
