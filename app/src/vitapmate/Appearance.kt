package vitapmate

import vitapmate.ui.rememberHaptics
import android.os.Build
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Moon
import com.composables.icons.lucide.Vibrate
import com.composables.icons.lucide.Palette as PaletteIcon
import com.composables.icons.lucide.PanelBottom
import com.composables.icons.lucide.Sun
import com.composables.icons.lucide.SunMoon
import com.composables.icons.lucide.Wallpaper
import vitapmate.data.Settings
import vitapmate.data.ThemeMode
import vitapmate.observability.Log
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.Palette
import vitapmate.ui.appColorScheme
import vitapmate.ui.showToast

/** Settings → Appearance: light/dark, the colours (wallpaper or a palette) and the nav bar's tabs. */
@Composable
fun AppearanceGroup() {
    val nav = LocalNav.current
    val haptics = rememberHaptics()
    ListGroup("Appearance") {
        GroupSegment(Modifier.testTag("settings.theme.row")) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(40.dp).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape), Alignment.Center) {
                    Icon(Lucide.SunMoon, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSecondaryContainer)
                }
                Text("Theme", Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                ConnectedChoice(
                    ThemeMode.entries,
                    Settings.themeMode,
                    onSelect = { mode ->
                        Settings.themeMode = mode
                        Log.info("ui.settings", "theme ${mode.name.lowercase()}")
                    },
                    tag = "settings.theme",
                    key = { it.name.lowercase() },
                    label = { if (it == ThemeMode.System) "Follow the system" else it.name },
                    icon = {
                        when (it) {
                            ThemeMode.System -> Lucide.SunMoon
                            ThemeMode.Light -> Lucide.Sun
                            ThemeMode.Dark -> Lucide.Moon
                        }
                    },
                    iconOnly = true,
                )
            }
        }
        GroupItem(
            "Pure black",
            "settings.pure_black",
            Lucide.Moon,
            "In dark mode",
            onClick = { Settings.pureBlack = !Settings.pureBlack; haptics.toggle(Settings.pureBlack) },
            trailing = { Switch(Settings.pureBlack, { Settings.pureBlack = it; haptics.toggle(it) }) },
        )
        GroupItem(
            "Haptics",
            "settings.haptics",
            Lucide.Vibrate,
            onClick = { Settings.haptics = !Settings.haptics; haptics.toggle(Settings.haptics) },
            trailing = { Switch(Settings.haptics, { Settings.haptics = it; haptics.toggle(it) }) },
        )
        GroupSegment {
            Label("Colours", Lucide.PaletteIcon)
            val choice = Settings.colorChoice
            val wallpaper = choice == Settings.WALLPAPER
            val selected = if (wallpaper) null else Palette.entries.firstOrNull { it.name == choice } ?: Palette.Default
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    Swatch("Wallpaper", "settings.colors.wallpaper", wallpaper, { pick(Settings.WALLPAPER) }) {
                        Box(Modifier.size(48.dp).background(MaterialTheme.colorScheme.primaryContainer, CircleShape), Alignment.Center) {
                            Icon(Lucide.Wallpaper, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onPrimaryContainer)
                        }
                    }
                }
                val dark = MaterialTheme.colorScheme.background.luminance() < 0.5f
                Palette.entries.forEach { p ->
                    Swatch(p.label, "settings.colors.${p.name.lowercase()}", p == selected, { pick(p.name) }) {
                        val scheme = remember(p, dark) { appColorScheme(dark, p) }
                        PaletteDot(scheme.primary, scheme.surfaceContainerHigh, p.accent)
                    }
                }
            }
        }
        GroupItem(
            "Tabs in the bar",
            "settings.tabs",
            Lucide.PanelBottom,
            navTabs.mapNotNull { featureById(it)?.label }.joinToString(", "),
            onClick = { nav.push(Route.Tabs) },
            trailing = { Chevron() },
        )
    }
}

private fun pick(choice: String) {
    Settings.colorChoice = choice
    Log.info("ui.settings", "colors $choice")
}

/** A segment's heading: an icon in a tonal circle and [text], like a [GroupItem]'s. */
@Composable
private fun Label(text: String, icon: ImageVector) = Row(
    Modifier.padding(bottom = 12.dp),
    horizontalArrangement = Arrangement.spacedBy(12.dp),
    verticalAlignment = Alignment.CenterVertically,
) {
    Box(Modifier.size(40.dp).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape), Alignment.Center) {
        Icon(icon, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSecondaryContainer)
    }
    Text(text, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurface)
}

/** A palette as a circle: its primary on top, a surface below with its accent as a dot. */
@Composable
private fun PaletteDot(primary: Color, surface: Color, accent: Color) = Box(Modifier.size(48.dp).clip(CircleShape).background(surface)) {
    Box(Modifier.fillMaxWidth().size(24.dp).background(primary))
    Box(Modifier.align(Alignment.BottomCenter).padding(bottom = 7.dp).size(10.dp).background(accent, CircleShape))
}

/** One colour choice: [preview], a ring and check when [selected], the [label] under it. */
@Composable
private fun Swatch(label: String, tag: String, selected: Boolean, onClick: () -> Unit, preview: @Composable () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Column(
        Modifier.clip(MaterialTheme.shapes.medium).selectable(selected, role = Role.RadioButton, onClick = onClick).padding(4.dp).testTag(tag),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.border(2.dp, if (selected) scheme.primary else Color.Transparent, CircleShape).padding(4.dp), Alignment.Center) {
            preview()
            if (selected) {
                Box(Modifier.size(20.dp).align(Alignment.TopEnd).background(scheme.primary, CircleShape), Alignment.Center) {
                    Icon(Lucide.Check, null, Modifier.size(12.dp), tint = scheme.onPrimary)
                }
            }
        }
        Text(label, Modifier.padding(top = 4.dp), style = MaterialTheme.typography.labelMedium, color = if (selected) scheme.onSurface else scheme.onSurfaceVariant)
    }
}
