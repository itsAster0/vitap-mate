package vitapmate.ui

import androidx.compose.runtime.getValue
import androidx.compose.foundation.layout.padding
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.animation.fadeOut
import androidx.compose.animation.fadeIn
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.background
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ButtonGroupDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.ToggleButton
import androidx.compose.material3.ToggleButtonDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow

/** Side padding of the buttons when a label would not fit the default padding. */
private val TightPadding = 6.dp

/**
 * One of [options], as M3 Expressive connected toggle buttons sharing the width; the selected one
 * fills with the primary colour and rounds fully. Each button is tagged `<tag>.<key>`. With
 * [iconOnly] the buttons show just the icon (the label is read out) and take only their size.
 * With [labelOnSelected] every button shows its icon and only the selected one adds its label,
 * growing with a spring while the others shrink. Labels never truncate: when space is short the
 * icons drop, then the padding tightens, then the buttons take widths by their labels. [badge]
 * puts a count (when above 0) in a small accent pill after a button's content.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun <T> ConnectedChoice(
    options: List<T>,
    selected: T,
    onSelect: (T) -> Unit,
    tag: String,
    key: (T) -> String,
    label: (T) -> String,
    modifier: Modifier = Modifier,
    icon: ((T) -> ImageVector)? = null,
    iconOnly: Boolean = false,
    labelOnSelected: Boolean = false,
    badge: ((T) -> Int)? = null,
) {
    val style = MaterialTheme.typography.labelLarge
    val measurer = rememberTextMeasurer()
    val density = LocalDensity.current
    val direction = LocalLayoutDirection.current
    val gap = ButtonGroupDefaults.ConnectedSpaceBetween
    BoxWithConstraints(if (iconOnly) modifier else modifier.fillMaxWidth()) {
        val texts = if (iconOnly) options.map { 0.dp } else options.map { with(density) { (measurer.measure(label(it), style, maxLines = 1, softWrap = false).size.width + 1).toDp() } }
        val slot = (maxWidth - gap * (options.size - 1)) / options.size
        val iconSpace = ToggleButtonDefaults.IconSize + ToggleButtonDefaults.IconSpacing
        val roomy = ButtonDefaults.SmallContentPadding
        val sides = roomy.calculateLeftPadding(direction) + roomy.calculateRightPadding(direction)
        val icons = icon != null && texts.all { it + sides + iconSpace <= slot }
        val fits = iconOnly || texts.all { it + sides + (if (icons) iconSpace else 0.dp) <= slot }
        val even = fits || texts.all { it + TightPadding * 2 <= slot }
        val haptics = rememberHaptics()
        val tones = AppTheme.tones
        val count = @Composable { option: T ->
            badge?.invoke(option)?.takeIf { it > 0 }?.let {
                Text(
                    "$it",
                    Modifier.padding(start = ToggleButtonDefaults.IconSpacing).background(tones.accent, CircleShape).widthIn(min = 18.dp).padding(horizontal = 5.dp),
                    style = MaterialTheme.typography.labelSmall.nums(fontWeight = FontWeight.Bold),
                    color = tones.onAccent,
                    textAlign = TextAlign.Center,
                    maxLines = 1,
                )
            }
        }
        if (labelOnSelected && icon != null) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(gap)) {
                options.forEachIndexed { i, option ->
                    val on = option == selected
                    val weight by animateFloatAsState(if (on) 2.6f else 1f, MaterialTheme.motionScheme.fastSpatialSpec(), label = "choice.weight")
                    ToggleButton(
                        checked = on,
                        onCheckedChange = { haptics.tick(); onSelect(option) },
                        modifier = Modifier.weight(weight).semantics { role = Role.RadioButton }.testTag("$tag.${key(option)}"),
                        shapes = when (i) {
                            0 -> ButtonGroupDefaults.connectedLeadingButtonShapes()
                            options.lastIndex -> ButtonGroupDefaults.connectedTrailingButtonShapes()
                            else -> ButtonGroupDefaults.connectedMiddleButtonShapes()
                        },
                        contentPadding = PaddingValues(horizontal = TightPadding),
                    ) {
                        Icon(icon(option), if (on) null else label(option), Modifier.size(ToggleButtonDefaults.IconSize))
                        AnimatedVisibility(on, enter = expandHorizontally() + fadeIn(), exit = shrinkHorizontally() + fadeOut()) {
                            Text(label(option), Modifier.padding(start = ToggleButtonDefaults.IconSpacing), maxLines = 1, softWrap = false, style = style)
                        }
                        count(option)
                    }
                }
            }
            return@BoxWithConstraints
        }
        Row(if (iconOnly) Modifier else Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(gap)) {
            options.forEachIndexed { i, option ->
                ToggleButton(
                    checked = option == selected,
                    onCheckedChange = { haptics.tick(); onSelect(option) },
                    modifier = when {
                        iconOnly -> Modifier.size(width = 56.dp, height = 40.dp)
                        even -> Modifier.weight(1f)
                        else -> Modifier.weight((texts[i] + TightPadding * 2).value)
                    }
                        .semantics { role = Role.RadioButton }
                        .testTag("$tag.${key(option)}"),
                    shapes = when (i) {
                        0 -> ButtonGroupDefaults.connectedLeadingButtonShapes()
                        options.lastIndex -> ButtonGroupDefaults.connectedTrailingButtonShapes()
                        else -> ButtonGroupDefaults.connectedMiddleButtonShapes()
                    },
                    contentPadding = if (fits) roomy else PaddingValues(horizontal = TightPadding),
                ) {
                    if (iconOnly && icon != null) {
                        Icon(icon(option), label(option), Modifier.size(ToggleButtonDefaults.IconSize))
                        return@ToggleButton
                    }
                    if (icons) {
                        Icon(icon(option), null, Modifier.size(ToggleButtonDefaults.IconSize))
                        Spacer(Modifier.size(ToggleButtonDefaults.IconSpacing))
                    }
                    Text(label(option), maxLines = 1, softWrap = false, overflow = TextOverflow.Visible, style = style)
                    count(option)
                }
            }
        }
    }
}
