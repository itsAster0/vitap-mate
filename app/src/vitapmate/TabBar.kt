package vitapmate

import androidx.compose.ui.graphics.Color
import com.composables.icons.lucide.History
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CloudOff
import com.composables.icons.lucide.Lucide
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import vitapmate.data.Activity
import vitapmate.data.OtpPrompt
import vitapmate.data.SemesterChoice
import vitapmate.ui.AppTheme
import vitapmate.ui.Avatar
import vitapmate.ui.Feature
import vitapmate.ui.PdfMenu
import vitapmate.ui.rememberHaptics
import vitapmate.ui.rememberNow
import vitapmate.ui.seasonIcon
import vitapmate.ui.shortSemesterName

/** The signed-in username, for the avatar; null while unknown. */
val LocalUser = staticCompositionLocalOf<String?> { null }

/** "Refreshing attendance" as "Attendance": the spinner next to it already says it is loading, and the bar is short. */
private fun thing(label: String) = label.removePrefix("Refreshing ").removePrefix("Loading ").replaceFirstChar { it.uppercase() }

/**
 * The left of a tab's app bar: today's date, or what is running ("Attendance" with a spinner,
 * "+2" when more run); tapping opens [ActivitySheet]. After it, a cloud-off icon while offline.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun TabBarLead() {
    var activity by rememberSaveable { mutableStateOf(false) }
    val now by rememberNow()
    val running = Activity.running.lastOrNull()
    val label = running?.let { thing(it.label) + (Activity.running.size - 1).let { more -> if (more > 0) " +$more" else "" } }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(
            Modifier.clip(MaterialTheme.shapes.large).clickable { activity = true }.padding(horizontal = 8.dp, vertical = 6.dp).testTag("bar.activity"),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            AnimatedContent(
                label,
                transitionSpec = { (slideInVertically { it / 2 } + fadeIn()) togetherWith (slideOutVertically { -it / 2 } + fadeOut()) },
                label = "bar-lead",
            ) { text ->
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (text != null) LoadingIndicator(Modifier.size(20.dp))
                    Text(
                        text ?: remember(now / 60_000) { SimpleDateFormat("EEE, d MMM", Locale.getDefault()).format(Date(now)) },
                        style = MaterialTheme.typography.titleMedium,
                        color = if (text != null) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
        AnimatedVisibility(Activity.offline && running == null, enter = scaleIn() + fadeIn(), exit = scaleOut() + fadeOut()) {
            IconButton({ activity = true }, Modifier.testTag("bar.offline")) {
                Icon(Lucide.CloudOff, "Offline", Modifier.size(20.dp), tint = AppTheme.tones.warning.base)
            }
        }
    }
    if (activity) ActivitySheet { activity = false }
}

/**
 * A pushed feature's title as the activity queue, like [TabBarLead]: [title], or what runs now
 * ("Attendance") with a spinner; tapping opens [ActivitySheet]. Takes the bar's text style (large or collapsed).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ActivityTitle(title: String) {
    var activity by rememberSaveable { mutableStateOf(false) }
    val running = Activity.running.lastOrNull()?.label?.let(::thing)
    Box(Modifier.clip(MaterialTheme.shapes.large).clickable { activity = true }.testTag("bar.activity")) {
        AnimatedContent(
            running,
            transitionSpec = { (slideInVertically { it / 2 } + fadeIn()) togetherWith (slideOutVertically { -it / 2 } + fadeOut()) },
            label = "bar-title",
        ) { label ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                if (label != null) LoadingIndicator(Modifier.size(24.dp))
                Text(
                    label ?: title,
                    color = if (label != null) MaterialTheme.colorScheme.onSurfaceVariant else Color.Unspecified,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
    if (activity) ActivitySheet { activity = false }
}

/**
 * The account button: the username's first letter in a circle, with a dot when something needs
 * attention (accent: the OTP is waiting, part of signing in; warning: offline or the last refresh
 * failed). A long press on a [feature] with a PDF offers to share or print it ([PdfMenu]).
 */
@Composable
fun AccountButton(feature: Feature?, onClick: () -> Unit) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    val haptics = rememberHaptics()
    var menu by remember { mutableStateOf(false) }
    val pdf = feature?.pdf
    val dot = when {
        OtpPrompt.message != null -> tones.accentTone.base
        Activity.offline || Activity.recent.firstOrNull()?.problem != null -> tones.warning.base
        else -> null
    }
    Box {
        Box(
            Modifier.size(48.dp).clip(CircleShape)
                .combinedClickable(
                    onClickLabel = "Settings",
                    onLongClickLabel = if (pdf != null) "Share or print as PDF" else null,
                    onLongClick = pdf?.let { { haptics.tick(); menu = true } },
                    onClick = onClick,
                )
                .testTag("header.account"),
            Alignment.Center,
        ) {
            Box {
                Avatar(LocalUser.current?.firstOrNull()?.uppercase().orEmpty(), 32.dp)
                if (dot != null) {
                    Box(Modifier.align(Alignment.TopEnd).size(10.dp).border(2.dp, scheme.background, CircleShape).padding(2.dp).background(dot, CircleShape))
                }
            }
        }
        if (pdf != null) PdfMenu(feature.id, menu, { menu = false }, pdf)
    }
}

/**
 * A small season icon (snowflake, leaf, sun) of the semester shown, only while VTOP lists a newer
 * one (the Flutter app's "New semester data available!"); tapping opens the semester sheet.
 */
@Composable
fun SemesterButton() {
    var open by rememberSaveable { mutableStateOf(false) }
    val list = SemesterChoice.semesters
    val other = list?.takeIf { it.selected != null && it.items.firstOrNull()?.id != it.selected }?.items?.firstOrNull { it.id == list.selected }
    AnimatedVisibility(other != null, enter = scaleIn() + fadeIn(), exit = scaleOut() + fadeOut()) {
        val shown = other ?: return@AnimatedVisibility
        IconButton({ open = true }, Modifier.testTag("bar.semester")) {
            Box(Modifier.size(32.dp).background(AppTheme.tones.warning.subtle, CircleShape), Alignment.Center) {
                Icon(seasonIcon(shown.season), "Showing ${shortSemesterName(shown)}; a newer semester is out", Modifier.size(18.dp), tint = AppTheme.tones.warning.onSubtle)
            }
        }
    }
    if (open) SemesterPickerSheet { open = false }
}
