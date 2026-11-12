package vitapmate

import vitapmate.ui.rememberHaptics
import android.os.Build
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.ZoomIn
import vitapmate.data.DISPLAY_SIZES
import vitapmate.ui.ConnectedChoice
import com.composables.icons.lucide.Moon
import com.composables.icons.lucide.Smile
import com.composables.icons.lucide.Vibrate
import com.composables.icons.lucide.Palette as PaletteIcon
import com.composables.icons.lucide.PanelBottom
import com.composables.icons.lucide.SunMoon
import com.composables.icons.lucide.Wallpaper
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Matrix
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.asComposePath
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.graphics.shapes.Morph
import androidx.graphics.shapes.toPath
import vitapmate.ui.AppTheme
import vitapmate.data.Settings
import vitapmate.data.ThemeMode
import vitapmate.observability.Log
import vitapmate.ui.Palette
import vitapmate.ui.appColorScheme
import vitapmate.ui.showToast

/** Settings → Appearance: light/dark, the colours (wallpaper or a palette), the app icon ([AppIcon]) and the nav bar's tabs. */
@Composable
fun AppearanceGroup() {
    val nav = LocalNav.current
    val haptics = rememberHaptics()
    ListGroup("Appearance") {
        GroupSegment(Modifier.testTag("settings.theme.row")) {
            Label("Theme", Lucide.SunMoon)
            val context = LocalContext.current
            val palette = Palette.entries.firstOrNull { it.name == Settings.colorChoice } ?: Palette.Default
            val wallpaper = Settings.colorChoice == Settings.WALLPAPER && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
            val light = if (wallpaper) dynamicLightColorScheme(context) else remember(palette) { appColorScheme(false, palette) }
            val dark = if (wallpaper) dynamicDarkColorScheme(context) else remember(palette, Settings.pureBlack) { appColorScheme(true, palette, Settings.pureBlack) }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                ThemeMode.entries.forEach { mode ->
                    val selected = mode == Settings.themeMode
                    val scheme = MaterialTheme.colorScheme
                    Box(Modifier.weight(1f)) {
                        Column(
                            Modifier.fillMaxWidth().clip(RoundedCornerShape(18.dp)).background(scheme.surfaceContainerHigh)
                                .border(2.dp, if (selected) scheme.primary else Color.Transparent, RoundedCornerShape(18.dp))
                                .selectable(selected, role = Role.RadioButton) {
                                    haptics.tick()
                                    Settings.themeMode = mode
                                    Log.info("ui.settings", "theme ${mode.name.lowercase()}")
                                }
                                .semantics { contentDescription = if (mode == ThemeMode.System) "Follow the system" else mode.name }
                                .testTag("settings.theme.${mode.name.lowercase()}")
                                .padding(start = 8.dp, top = 8.dp, end = 8.dp, bottom = 9.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            ThemePreview(
                                when (mode) {
                                    ThemeMode.System -> listOf(light, dark)
                                    ThemeMode.Light -> listOf(light)
                                    ThemeMode.Dark -> listOf(dark)
                                },
                            )
                            Text(
                                mode.name,
                                Modifier.padding(top = 7.dp).clearAndSetSemantics {},
                                style = MaterialTheme.typography.labelMedium,
                                color = if (selected) scheme.onSurface else scheme.onSurfaceVariant,
                            )
                        }
                        if (selected) Check(Modifier.align(Alignment.TopEnd).offset(5.dp, (-5).dp))
                    }
                }
            }
        }
        // Only matters while the app is dark (Dark, or System with the system dark).
        if (MaterialTheme.colorScheme.background.luminance() < 0.5f) GroupItem(
            "Pure black",
            "settings.pure_black",
            Lucide.Moon,
            "In dark mode",
            onClick = { Settings.pureBlack = !Settings.pureBlack; haptics.toggle(Settings.pureBlack) },
            trailing = { ItemSwitch(Settings.pureBlack, { Settings.pureBlack = it; haptics.toggle(it) }) },
            on = Settings.pureBlack,
        )
        GroupItem(
            "Haptics",
            "settings.haptics",
            Lucide.Vibrate,
            onClick = { Settings.haptics = !Settings.haptics; haptics.toggle(Settings.haptics) },
            trailing = { ItemSwitch(Settings.haptics, { Settings.haptics = it; haptics.toggle(it) }) },
            on = Settings.haptics,
        )
        GroupSegment(Modifier.testTag("settings.display_size.row")) {
            Label("Display size", Lucide.ZoomIn)
            ConnectedChoice(
                DISPLAY_SIZES,
                Settings.displaySize,
                onSelect = { size ->
                    Settings.displaySize = size
                    Log.info("ui.settings", "display size $size")
                },
                tag = "settings.display_size",
                key = { "$it" },
                label = { when (it) { 0 -> "System"; 90 -> "Small"; 100 -> "Medium"; else -> "Large" } },
            )
        }
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
                    Swatch(p.label, "settings.colors.${p.name.lowercase()}", p == selected, { pick(p.name) }, ring = null) {
                        val scheme = remember(p, dark) { appColorScheme(dark, p) }
                        val cookie by animateFloatAsState(if (p == selected) 1f else 0f, MaterialTheme.motionScheme.defaultSpatialSpec())
                        PaletteDot(scheme.primary, scheme.surfaceContainerHigh, p.accent, remember(cookie) { MorphShape(cookieMorph, cookie) })
                    }
                }
            }
        }
        GroupSegment {
            Label("App icon", Lucide.Smile)
            val context = LocalContext.current
            var icon by remember { mutableStateOf(AppIcon.current(context)) }
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                AppIcon.entries.forEach { i ->
                    Swatch(i.name, "settings.icon.${i.name.lowercase()}", i == icon, ring = RoundedCornerShape(21.dp), onClick = {
                        if (i != icon) {
                            AppIcon.set(context, i)
                            icon = i
                            haptics.tick()
                            Log.info("ui.settings", "icon ${i.name.lowercase()}")
                            showToast("App icon changed", "Your home screen may take a moment to show it.")
                        }
                    }) {
                        // The adaptive icon's layers: the foreground's middle two thirds fill the squircle.
                        val squircle = RoundedCornerShape(17.dp)
                        Box(Modifier.size(48.dp).clip(squircle).background(i.background).border(1.dp, MaterialTheme.colorScheme.outlineVariant, squircle), Alignment.Center) {
                            Image(
                                painterResource(if (i.lightHead) R.drawable.ic_launcher_foreground_light else R.drawable.ic_launcher_foreground),
                                null,
                                Modifier.requiredSize(72.dp),
                            )
                        }
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
            below = { BarPreview(Modifier.padding(top = 7.dp)) },
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

/** A palette in [shape]: its primary on top, a surface below with its accent as a dot. */
@Composable
private fun PaletteDot(primary: Color, surface: Color, accent: Color, shape: Shape) = Box(Modifier.size(48.dp).clip(shape).background(surface)) {
    Box(Modifier.fillMaxWidth().size(24.dp).background(primary))
    Box(Modifier.align(Alignment.BottomCenter).padding(bottom = 7.dp).size(10.dp).background(accent, CircleShape))
}

/** The chosen palette's circle turns into a cookie. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
private val cookieMorph by lazy { Morph(MaterialShapes.Circle, MaterialShapes.Cookie12Sided) }

/** [morph] at [progress], scaled to the size (Material's shapes are in a unit square). */
private class MorphShape(private val morph: Morph, private val progress: Float) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline {
        val path = morph.toPath(progress).asComposePath()
        path.transform(Matrix().apply { scale(size.width, size.height) })
        return Outline.Generic(path)
    }
}

/** A theme as a tiny screen: its background with three bars (the first in its primary); two schemes split diagonally. */
@Composable
private fun ThemePreview(schemes: List<ColorScheme>) = Canvas(Modifier.fillMaxWidth().height(54.dp).clip(RoundedCornerShape(11.dp))) {
    val pad = 8.dp.toPx()
    val bar = 8.dp.toPx()
    val draw = { scheme: ColorScheme ->
        drawRect(scheme.background)
        repeat(3) { r ->
            val width = (size.width - pad * 2) * if (r == 0) 0.55f else 1f
            drawRoundRect(if (r == 0) scheme.primary else scheme.surfaceContainerHigh, Offset(pad, pad + r * (bar + 4.dp.toPx())), Size(width, bar), CornerRadius(bar / 2))
        }
    }
    schemes.forEachIndexed { i, scheme ->
        if (i == 0) draw(scheme)
        else clipPath(Path().apply { moveTo(size.width * 0.62f, 0f); lineTo(size.width, 0f); lineTo(size.width, size.height); lineTo(size.width * 0.38f, size.height); close() }) { draw(scheme) }
    }
}

/** The chosen mark: a check in a primary circle ringed with the card colour. */
@Composable
private fun Check(modifier: Modifier = Modifier) = Box(
    modifier.size(20.dp).background(MaterialTheme.colorScheme.primary, CircleShape).border(2.dp, AppTheme.colors.card, CircleShape),
    Alignment.Center,
) { Icon(Lucide.Check, null, Modifier.size(12.dp), tint = MaterialTheme.colorScheme.onPrimary) }

/** One colour choice: [preview], a [ring] (if any) and check when [selected], the [label] under it. */
@Composable
private fun Swatch(label: String, tag: String, selected: Boolean, onClick: () -> Unit, ring: Shape? = CircleShape, preview: @Composable () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Column(
        Modifier.clip(MaterialTheme.shapes.medium).selectable(selected, role = Role.RadioButton, onClick = onClick).padding(4.dp).testTag(tag),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.border(2.dp, if (selected && ring != null) scheme.primary else Color.Transparent, ring ?: CircleShape).padding(4.dp), Alignment.Center) {
            preview()
            if (selected) Check(Modifier.align(Alignment.TopEnd))
        }
        Text(label, Modifier.padding(top = 4.dp), style = MaterialTheme.typography.labelMedium, color = if (selected) scheme.onSurface else scheme.onSurfaceVariant)
    }
}
