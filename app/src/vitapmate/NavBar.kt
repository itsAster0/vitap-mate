package vitapmate

import vitapmate.ui.rememberHaptics
import android.view.HapticFeedbackConstants
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.FloatingToolbarScrollBehavior
import androidx.compose.material3.HorizontalFloatingToolbar
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.ui.draw.clip
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import vitapmate.ui.Feature

/**
 * The floating nav bar: an M3 Expressive floating toolbar with one item per tab. The selected
 * tab grows into a pill with its label; it slides away on scroll down and back on scroll up.
 * Tapping the current tab again calls [onSelect] too (screens scroll to the top); a long press
 * on any tab opens the tab editor ([onEdit]).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun NavBar(
    tabs: List<Feature>,
    selected: String?,
    scroll: FloatingToolbarScrollBehavior,
    onSelect: (String) -> Unit,
    onEdit: () -> Unit,
    modifier: Modifier = Modifier,
) {
    HorizontalFloatingToolbar(
        expanded = true,
        modifier = modifier.testTag("nav.bar"),
        colors = FloatingToolbarDefaults.standardFloatingToolbarColors().copy(toolbarContainerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        scrollBehavior = scroll,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
            tabs.forEach { NavItem(it, it.id == selected, { onSelect(it.id) }, onEdit) }
        }
    }
}

@Composable
private fun NavItem(feature: Feature, selected: Boolean, onClick: () -> Unit, onLongClick: () -> Unit) {
    val haptics = rememberHaptics()
    val scheme = MaterialTheme.colorScheme
    val container by animateColorAsState(if (selected) scheme.primaryContainer else Color.Transparent)
    val content by animateColorAsState(if (selected) scheme.onPrimaryContainer else scheme.onSurfaceVariant)
    Row(
        Modifier
            .height(48.dp)
            .widthIn(min = 56.dp)
            .clip(CircleShape)
            .background(container)
            .combinedClickable(
                onClickLabel = feature.label,
                onLongClickLabel = "Edit tabs",
                onLongClick = {
                    haptics.longPress()
                    onLongClick()
                },
            ) {
                haptics.tick()
                onClick()
            }
            .padding(horizontal = 16.dp)
            .testTag("tab.${feature.id}")
            .semantics {
                role = Role.Tab
                this.selected = selected
                contentDescription = feature.label
            },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
    ) {
        CompositionLocalProvider(LocalContentColor provides content) {
            Icon(feature.icon, null, Modifier.size(22.dp))
            AnimatedVisibility(selected, enter = fadeIn() + expandHorizontally(), exit = fadeOut() + shrinkHorizontally()) {
                Text(feature.label, Modifier.padding(start = 8.dp), style = MaterialTheme.typography.labelLarge, maxLines = 1)
            }
        }
    }
}
