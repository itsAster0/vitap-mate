package vitapmate

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.runtime.staticCompositionLocalOf
import vitapmate.data.NavTabs
import vitapmate.data.Settings

/** A place in the app: a feature or built-in screen by id, the tab editor, or one flow's log lines. */
sealed interface Route {
    val key: String

    data class Screen(val id: String) : Route {
        override val key get() = id
    }

    data object Tabs : Route {
        override val key get() = "settings.tabs"
    }

    data class FlowLog(val flow: String) : Route {
        override val key get() = "flow:$flow"
    }

    /** A feature's detail screen ([vitapmate.ui.Feature.detail]) for [item], titled [title]. */
    data class Detail(val feature: String, val item: String, val title: String) : Route {
        override val key get() = "detail:" + listOf(feature, item, title).joinToString("\u0000")
    }

    companion object {
        fun parse(key: String): Route = when {
            key == Tabs.key -> Tabs
            key.startsWith("flow:") -> FlowLog(key.removePrefix("flow:"))
            key.startsWith("detail:") -> key.removePrefix("detail:").split("\u0000").let { Detail(it[0], it.getOrElse(1) { "" }, it.getOrElse(2) { "" }) }
            else -> Screen(key)
        }
    }
}

/** Bar tabs: the saved ids that still exist (core keeps at most 5 and always More). */
val navTabs: List<String>
    get() = Settings.navTabs.filter { id -> tabChoices.any { it.id == id } }.ifEmpty { NavTabs.DEFAULT }

/**
 * The back stack. Its root is a bar tab; everything opened from there is pushed on top. Survives process death as route keys.
 */
@Stable
class Nav(keys: List<String>) {
    val stack = mutableStateListOf<Route>().apply { addAll(keys.map(Route::parse)) }

    val top: Route get() = stack.last()
    val root: Route get() = stack.first()

    /** The tab shown as selected: the root, when it is in the bar. */
    val tab: String? get() = (root as? Route.Screen)?.id?.takeIf { it in navTabs }

    fun goTab(id: String) = Snapshot.withMutableSnapshot {
        stack.clear()
        stack.add(Route.Screen(id))
    }

    fun push(route: Route) {
        if (top != route) stack.add(route)
    }

    fun pop() {
        if (stack.size > 1) stack.removeAt(stack.lastIndex)
    }

    companion object {
        val saver = listSaver<Nav, String>({ nav -> nav.stack.map { it.key } }, ::Nav)
    }
}

@Composable
fun rememberNav(): Nav = rememberSaveable(saver = Nav.saver) { Nav(listOf(navTabs.first())) }

val LocalNav = staticCompositionLocalOf<Nav> { error("no Nav") }

/**
 * The route of the entry being drawn. Screens ask it, not the stack, whether they are the root:
 * during back the page below is drawn while the stack still holds the one leaving.
 */
val LocalRoute = staticCompositionLocalOf<Route?> { null }
