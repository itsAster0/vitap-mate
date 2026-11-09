package vitapmate

import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.material3.CircularProgressIndicator
import com.composables.icons.lucide.CloudDownload
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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
import com.composables.icons.lucide.Pin
import vitapmate.ui.AppTheme
import vitapmate.ui.Feature
import vitapmate.ui.tappable

/** Tiles per row in More's feature grid. */
private const val COLUMNS = 3

/**
 * Every feature as a grid of tiles, the ones in the bar first and pinned (tapping one switches
 * to its tab), then Settings, the Inspector and the version.
 */
@Composable
fun MoreScreen() {
    val nav = LocalNav.current
    val tabs = navTabs
    val version = rememberAppVersion()
    val ordered = features.sortedBy { f -> tabs.indexOf(f.id).let { if (it < 0) Int.MAX_VALUE else it } }
    Page("More", MORE) { padding ->
        LazyColumn(contentPadding = padding) {
            item("features") {
                Column(Modifier.padding(horizontal = 16.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Features",
                        Modifier.padding(start = 16.dp).semantics { heading() },
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    ordered.chunked(COLUMNS).forEach { row ->
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            row.forEach { f ->
                                val pinned = f.id in tabs
                                FeatureTile(f, pinned, { if (pinned) nav.goTab(f.id) else nav.push(Route.Screen(f.id)) }, Modifier.weight(1f))
                            }
                            repeat(COLUMNS - row.size) { Spacer(Modifier.weight(1f)) }
                        }
                    }
                }
            }
            item("app") {
                ListGroup("App") {
                    val context = LocalContext.current
                    GroupItem(
                        "Fetch all",
                        "more.fetch_all",
                        Lucide.CloudDownload,
                        FetchAll.status ?: "Refresh every VTOP page now",
                        onClick = { FetchAll.start(context) },
                        trailing = { if (FetchAll.running) CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) },
                    )
                    InboxRow()
                    builtIns.filter { it.id != MORE && it.id != INSPECTOR }.forEach { f ->
                        GroupItem(f.label, "more.${f.id}", f.icon, onClick = { nav.push(Route.Screen(f.id)) }, trailing = { Chevron() })
                    }
                    GroupItem("About", "more.about", Lucide.Info, "vitapmate", onClick = rememberDeveloperTaps(), trailing = { version?.let { TrailingValue(it) } })
                }
            }
        }
    }
}

/** A feature: its icon in a tonal cookie, its label; a pin when it is in the bar. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun FeatureTile(feature: Feature, pinned: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val card = AppTheme.colors.card
    Column(
        modifier.clip(MaterialTheme.shapes.large).background(card)
            .tappable(onClick, if (pinned) "${feature.label}, in the bar" else feature.label, Role.Button)
            .padding(horizontal = 8.dp, vertical = 16.dp)
            .testTag("more.${feature.id}"),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box {
            Box(Modifier.size(52.dp).background(scheme.secondaryContainer, MaterialShapes.Cookie7Sided.toShape()), Alignment.Center) {
                Icon(feature.icon, null, Modifier.size(24.dp), tint = scheme.onSecondaryContainer)
            }
            if (pinned) {
                Box(
                    Modifier.align(Alignment.TopEnd).size(20.dp).border(2.dp, card, CircleShape).padding(2.dp).background(scheme.primary, CircleShape),
                    Alignment.Center,
                ) {
                    Icon(Lucide.Pin, null, Modifier.size(9.dp), tint = scheme.onPrimary)
                }
            }
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
