package vitapmate

import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LoadingIndicator
import com.composables.icons.lucide.CloudDownload
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Unplug
import com.composables.icons.lucide.Pin
import vitapmate.ui.AppTheme
import vitapmate.ui.Feature
import vitapmate.ui.tappable

/** Tiles per row in More's feature grid. */
private const val COLUMNS = 3

/**
 * Every feature as tiles under its section ([featureGroups]), the ones in the bar pinned (tapping
 * one switches to its tab), then Settings, the Inspector and the version.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun MoreScreen() {
    val nav = LocalNav.current
    val tabs = navTabs
    val version = rememberAppVersion()
    Page("More", MORE) { padding ->
        LazyColumn(contentPadding = padding) {
            featureGroups.forEachIndexed { index, (title, group) ->
                item(title) {
                    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            title,
                            Modifier.padding(start = 16.dp).semantics { heading() },
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        // Each section's own icon shape.
                        val shape = when (index) {
                            1 -> MaterialShapes.Sunny.toShape()
                            2 -> MaterialShapes.Clover4Leaf.toShape()
                            3 -> RoundedCornerShape(17.dp)
                            else -> MaterialShapes.Cookie7Sided.toShape()
                        }
                        // One segmented block: the clip rounds its ends, tiles keep small corners.
                        Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                            group.chunked(COLUMNS).forEach { row ->
                                Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                                    row.forEach { f ->
                                        val pinned = f.id in tabs
                                        FeatureTile(f, pinned, shape, { if (pinned) nav.goTab(f.id) else nav.push(Route.Screen(f.id)) }, Modifier.weight(1f).fillMaxHeight())
                                    }
                                    repeat(COLUMNS - row.size) { Spacer(Modifier.weight(1f)) }
                                }
                            }
                        }
                    }
                }
            }
            item("app") {
                ListGroup("App") {
                    val context = LocalContext.current
                    InboxRow()
                    GroupItem(
                        "Fetch all",
                        "more.fetch_all",
                        Lucide.CloudDownload,
                        FetchAll.status ?: "Refresh every VTOP page now",
                        onClick = { FetchAll.start(context) },
                        trailing = { if (FetchAll.running) LoadingIndicator(Modifier.size(30.dp)) },
                        on = FetchAll.running,
                        below = { if (FetchAll.running) LinearWavyProgressIndicator(Modifier.fillMaxWidth().padding(top = 8.dp)) },
                    )
                    GroupItem("Connected apps", "more.connected_apps", Lucide.Unplug, "Browser extension and AI apps (MCP)", onClick = { nav.push(Route.Screen(CONNECTED_APPS)) }, trailing = { Chevron() })
                    // Settings when it isn't already in the bar.
                    builtIns.filter { it.id != MORE && it.id != INSPECTOR && it.id !in tabs }.forEach { f ->
                        GroupItem(f.label, "more.${f.id}", f.icon, onClick = { nav.push(Route.Screen(f.id)) }, trailing = { Chevron() })
                    }
                    GroupItem("About", "more.about", Lucide.Info, version?.let { "vitapmate $it · what's new and credits" } ?: "What's new and credits", onClick = { nav.push(Route.Screen(UPDATES)) }, trailing = { Chevron() })
                }
            }
        }
    }
}

/** A feature: its icon in the section's [shape] (primary when it is in the bar, with a pin), its label. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun FeatureTile(feature: Feature, pinned: Boolean, shape: Shape, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Box(
        modifier.clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(AppTheme.colors.card)
            .tappable(onClick, if (pinned) "${feature.label}, in the bar" else feature.label, Role.Button)
            .testTag("more.${feature.id}"),
    ) {
        Column(Modifier.fillMaxWidth().padding(start = 4.dp, top = 18.dp, end = 4.dp, bottom = 14.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Box(Modifier.size(54.dp).background(if (pinned) scheme.primary else scheme.secondaryContainer, shape), Alignment.Center) {
                Icon(feature.icon, null, Modifier.size(24.dp), tint = if (pinned) scheme.onPrimary else scheme.onSecondaryContainer)
            }
            Text(
                feature.label,
                Modifier.fillMaxWidth().padding(top = 10.dp),
                style = MaterialTheme.typography.labelLarge,
                color = scheme.onSurface,
                textAlign = TextAlign.Center,
                maxLines = 1,
            )
        }
        if (pinned) {
            Box(Modifier.align(Alignment.TopEnd).padding(top = 10.dp, end = 12.dp).size(22.dp).background(tones.accentTone.subtle, CircleShape), Alignment.Center) {
                Icon(Lucide.Pin, null, Modifier.size(11.dp), tint = tones.accent)
            }
        }
    }
}

@Composable
fun Chevron() = Icon(Lucide.ChevronRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)

/** A row's value on the right (e.g. the version), in the muted colour. */
@Composable
fun TrailingValue(text: String) = Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

/** The installed version name and code ("1.4.0 (52)"); none for a build without a name. */
@Composable
fun rememberAppVersion(): String? {
    val context = LocalContext.current
    return remember {
        runCatching {
            val info = context.packageManager.getPackageInfo(context.packageName, 0)
            val code = if (android.os.Build.VERSION.SDK_INT >= 28) info.longVersionCode else @Suppress("DEPRECATION") info.versionCode.toLong()
            info.versionName?.takeIf { it != "unspecified" }?.let { "$it ($code)" }
        }.getOrNull()
    }
}

/**
 * Tapping the version 7 times in a row (each within 1.5 s of the last) turns Developer on, as
 * on Android's build number; it stays on until the switch at the top of Developer turns it off.
 */
@Composable
fun rememberDeveloperTaps(): () -> Unit {
    val haptics = rememberHaptics()
    val taps = remember { longArrayOf(0, 0) } // count, last tap ms
    return {
        val now = System.currentTimeMillis()
        taps[0] = if (now - taps[1] > 1_500) 1 else taps[0] + 1
        taps[1] = now
        val left = 7 - taps[0]
        when {
            left == 0L -> {
                taps[0] = 0
                haptics.confirm()
                if (Settings.developerEnabled) {
                    showToast("Developer is already on", "")
                } else {
                    Settings.developerEnabled = true
                    Log.info("ui.settings", "developer on")
                    showToast("Developer is on", "Under Diagnostics in Settings.")
                }
            }
            // No countdown toast: it would sit over the row being tapped.
            else -> haptics.tick()
        }
    }
}
