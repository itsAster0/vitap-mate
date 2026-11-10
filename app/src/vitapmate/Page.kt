package vitapmate

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.union
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowLeft
import com.composables.icons.lucide.Lucide
import vitapmate.ui.AppTopBar

/** Height the floating nav bar takes at the bottom (bar + its gap), excluding the gesture area. */
val BarSpace = 96.dp

/**
 * One screen in the shell, over [content] (which gets the padding that clears both bars; it may
 * draw under the floating bar, so lists end with that padding).
 *
 * A tab root has no title (the nav bar names it): a slim bar with [actions] and the account
 * button, which slides away on scroll down like the nav bar. A screen pushed on a tab gets a back
 * arrow and [title]; a feature (from More) keeps the semester and account buttons.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Page(
    title: String,
    tag: String,
    actions: @Composable RowScope.() -> Unit = {},
    content: @Composable (PaddingValues) -> Unit,
) {
    val nav = LocalNav.current
    // This page's own place, not the stack size: the tab root drawn under a back keeps its tab header.
    val root = (LocalRoute.current ?: nav.top) == nav.root
    // The tab bar follows the finger and stays where it's let go: the default snap pulled a
    // partly hidden bar (and the page under it) back down after a short scroll, a bounce.
    val scroll = if (root) TopAppBarDefaults.enterAlwaysScrollBehavior(snapAnimationSpec = null) else TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    Scaffold(
        Modifier.nestedScroll(scroll.nestedScrollConnection).testTag("screen.$tag"),
        topBar = {
            if (root) {
                TopAppBar(
                    title = { TabBarLead() },
                    actions = {
                        actions()
                        SemesterButton()
                        if (tag != SETTINGS) AccountButton(featureById(tag)) { nav.push(Route.Screen(SETTINGS)) }
                    },
                    scrollBehavior = scroll,
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.background,
                        scrolledContainerColor = MaterialTheme.colorScheme.background,
                    ),
                    modifier = Modifier.testTag("header"),
                )
            } else {
                val feature = tag != SETTINGS && featureById(tag) != null
                AppTopBar(
                    title,
                    navigationIcon = {
                        IconButton(nav::pop, Modifier.testTag("nav.back")) { Icon(Lucide.ArrowLeft, "Back") }
                    },
                    // A feature opened from More keeps the tab bar's semester and account buttons, and its
                    // title shows what runs now (tap: recent activity), like the tab bar's date.
                    actions = {
                        actions()
                        if (feature) {
                            SemesterButton()
                            AccountButton(featureById(tag)) { nav.push(Route.Screen(SETTINGS)) }
                        }
                    },
                    scrollBehavior = scroll,
                    titleContent = if (feature) ({ ActivityTitle(title) }) else null,
                )
            }
        },
        containerColor = MaterialTheme.colorScheme.background,
        contentWindowInsets = WindowInsets.statusBars.union(WindowInsets.displayCutout).only(WindowInsetsSides.Horizontal + WindowInsetsSides.Top),
    ) { padding ->
        val dir = LocalLayoutDirection.current
        val bottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding() +
            if (root) BarSpace else 16.dp
        Box {
            content(
                PaddingValues(
                    start = padding.calculateStartPadding(dir),
                    top = padding.calculateTopPadding(),
                    end = padding.calculateEndPadding(dir),
                    bottom = bottom,
                ),
            )
        }
    }
}
