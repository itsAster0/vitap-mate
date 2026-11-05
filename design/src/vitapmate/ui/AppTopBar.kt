package vitapmate.ui

import androidx.compose.foundation.layout.RowScope
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LargeFlexibleTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MediumFlexibleTopAppBar
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.TopAppBarScrollBehavior
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow

/**
 * The screen's title bar: an M3 flexible top app bar (medium, or [large]) that collapses with
 * [scrollBehavior], e.g. `TopAppBarDefaults.exitUntilCollapsedScrollBehavior()` with its
 * `nestedScrollConnection` on the screen. Page-coloured; it tints when content scrolls under it.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AppTopBar(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    large: Boolean = false,
    scrollBehavior: TopAppBarScrollBehavior? = null,
    navigationIcon: @Composable () -> Unit = {},
    actions: @Composable RowScope.() -> Unit = {},
    titleContent: (@Composable () -> Unit)? = null,
) {
    val colors = TopAppBarDefaults.topAppBarColors(
        containerColor = MaterialTheme.colorScheme.background,
        scrolledContainerColor = MaterialTheme.colorScheme.background,
    )
    val titleText = titleContent ?: @Composable { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) }
    val subtitleText = subtitle?.let { @Composable { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } }
    if (large) {
        LargeFlexibleTopAppBar(
            titleText, modifier.testTag("topbar"), subtitleText, navigationIcon = navigationIcon, actions = actions,
            colors = colors, scrollBehavior = scrollBehavior,
        )
    } else {
        MediumFlexibleTopAppBar(
            titleText, modifier.testTag("topbar"), subtitleText, navigationIcon = navigationIcon, actions = actions,
            colors = colors, scrollBehavior = scrollBehavior,
        )
    }
}
