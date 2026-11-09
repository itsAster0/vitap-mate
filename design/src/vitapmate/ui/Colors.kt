package vitapmate.ui

import androidx.compose.material3.ColorScheme
import com.materialkolor.PaletteStyle
import com.materialkolor.dynamicColorScheme
import com.materialkolor.ktx.from
import com.materialkolor.ktx.harmonize
import com.materialkolor.ktx.toneColor
import com.materialkolor.palettes.TonalPalette
import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/**
 * The only colours the app defines: seeds. Every role and tone is derived from them with
 * Material's tonal palettes (HCT), so light, dark and contrast stay consistent.
 */
private object Seed {
    /** A grey with no chroma, so surfaces stay neutral like the Flutter app. */
    val neutral = Color(0xFF777777)
    val success = Color(0xFF16A34A)
    val warning = Color(0xFFD97706)
    val danger = Color(0xFFDC2626)
    val lab = Color(0xFF0D9488)
}

/**
 * The app's colour choices (Settings → Appearance), all on neutral grey surfaces. [primary] seeds
 * buttons, selection and the nav bar; [accent] seeds `AppTheme.tones.accentTone` (what is live or
 * selected in content). [Graphite] is the Flutter app's look: black and white controls, an indigo
 * accent.
 */
enum class Palette(val label: String, val primary: Color, val accent: Color, val style: PaletteStyle) {
    Graphite("Graphite", Color(0xFF3F3F46), Color(0xFF4F46E5), PaletteStyle.Monochrome),
    Indigo("Indigo", Color(0xFF4F46E5), Color(0xFF4F46E5), PaletteStyle.TonalSpot),
    Ocean("Ocean", Color(0xFF0369A1), Color(0xFF0369A1), PaletteStyle.TonalSpot),
    Violet("Violet", Color(0xFF7C3AED), Color(0xFF7C3AED), PaletteStyle.TonalSpot),
    ;

    companion object {
        val Default = Graphite
    }
}

/** [palette]'s M3 scheme on neutral grey surfaces, [Seed.danger] as error; [pureBlack] for OLED. */
fun appColorScheme(dark: Boolean, palette: Palette = Palette.Default, pureBlack: Boolean = false): ColorScheme = dynamicColorScheme(
    seedColor = palette.primary,
    isDark = dark,
    isAmoled = dark && pureBlack,
    primary = palette.primary,
    neutral = Seed.neutral,
    neutralVariant = Seed.neutral,
    error = Seed.danger,
    style = palette.style,
)

/**
 * A [Tone] from a seed's tonal palette. [Tone.subtle] sits a step quieter than M3's container
 * tone (95 light, 25 dark) so tints read as a hint on neutral surfaces, like the Flutter app's.
 * In dark mode the fill keeps the seed's hue at low chroma (as M3's own dark containers do): a
 * vivid seed at tone 20 is a deep saturated block that shouts over the text on it.
 */
private fun toneOf(seed: Color, dark: Boolean): Tone {
    val palette = TonalPalette.from(seed)
    if (!dark) return Tone(palette.toneColor(40), palette.toneColor(95), palette.toneColor(20))
    val quiet = TonalPalette.fromHueAndChroma(palette.hue, minOf(palette.chroma, DARK_SUBTLE_CHROMA))
    return Tone(palette.toneColor(80), quiet.toneColor(25), palette.toneColor(90))
}

private const val DARK_SUBTLE_CHROMA = 16.0

fun ColorScheme.card(dark: Boolean): Color = if (dark) surfaceContainerLow else surfaceContainerLowest

/**
 * forui's neutral roles, read from the M3 scheme so wallpaper colours reach the older components
 * too. New code reads `MaterialTheme.colorScheme` instead.
 */
@Immutable
data class AppColors(
    val dark: Boolean,
    val barrier: Color,
    val background: Color,
    val foreground: Color,
    val primary: Color,
    val primaryForeground: Color,
    val secondary: Color,
    val secondaryForeground: Color,
    val muted: Color,
    val mutedForeground: Color,
    val destructive: Color,
    val destructiveForeground: Color,
    val error: Color,
    val errorForeground: Color,
    val card: Color,
    val border: Color,
) {
    companion object {
        fun of(s: ColorScheme, dark: Boolean) = AppColors(
            dark = dark,
            barrier = s.scrim.copy(alpha = if (dark) 0.48f else 0.2f),
            background = s.background,
            foreground = s.onBackground,
            primary = s.onSurface,
            primaryForeground = s.surface,
            secondary = s.surfaceContainerHigh,
            secondaryForeground = s.onSurface,
            muted = s.surfaceContainerHigh,
            mutedForeground = s.onSurfaceVariant,
            destructive = s.error,
            destructiveForeground = s.onError,
            error = s.error,
            errorForeground = s.onError,
            card = s.card(dark),
            border = s.outlineVariant,
        )
    }
}

/**
 * A colour role: a solid [base] for icons, bars and strokes, a quiet [subtle] fill for badges and
 * highlighted surfaces, and [onSubtle] for text placed on that fill.
 */
@Immutable
data class Tone(val base: Color, val subtle: Color, val onSubtle: Color)

/**
 * Status tones (`AppTheme.tones`): [accent] marks what is live or selected (it follows the M3
 * primary, so wallpaper colours change it), the others describe state and stay fixed; [lab]
 * separates lab sessions from theory.
 */
@Immutable
data class AppPalette(
    val accent: Color,
    val onAccent: Color,
    val accentTone: Tone,
    val success: Tone,
    val warning: Tone,
    val danger: Tone,
    val lab: Tone,
    /** Soft shadow for raised surfaces; transparent in dark mode, where borders carry elevation. */
    val shadow: Color,
) {
    companion object {
        /** Tones on [s]; [accent] seeds the accent tone (the scheme's primary for wallpaper colours). */
        fun of(s: ColorScheme, dark: Boolean, accent: Color = s.primary) = toneOf(accent, dark).let { tone ->
            AppPalette(
            accent = tone.base,
            onAccent = if (dark) s.surface else s.onPrimary,
            accentTone = tone,
            // Status colours keep their meaning but lean toward the accent's hue, so they belong
            // to whichever palette (or wallpaper) is chosen.
            success = toneOf(Seed.success.harmonize(accent), dark),
            warning = toneOf(Seed.warning.harmonize(accent), dark),
            danger = toneOf(Seed.danger.harmonize(accent), dark),
            lab = toneOf(Seed.lab.harmonize(accent), dark),
            shadow = if (dark) Color.Transparent else s.scrim.copy(alpha = 0.06f),
            )
        }
    }
}

/** New name for the status tones; same object as [AppPalette]. */
typealias Tones = AppPalette
