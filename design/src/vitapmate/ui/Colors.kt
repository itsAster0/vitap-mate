package vitapmate.ui

import android.content.Context
import androidx.annotation.RequiresApi
import androidx.compose.material3.ColorScheme
import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.toArgb
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.pow
import kotlin.math.roundToInt

/**
 * The app's colour choices (Settings → Appearance), all on neutral grey surfaces. [primary] seeds
 * buttons, selection and the nav bar; [accent] seeds `AppTheme.tones.accentTone` (what is live or
 * selected in content). [Graphite] is the Flutter app's look: black and white controls, an indigo
 * accent. Every role and tone comes from Material's tonal palettes (HCT) of these seeds, a grey
 * with no chroma for surfaces and fixed seeds for the status tones, precomputed in ColorTables.kt.
 */
enum class Palette(val label: String, val primary: Color, val accent: Color) {
    Graphite("Graphite", Color(0xFF3F3F46), Color(0xFF4F46E5)),
    Indigo("Indigo", Color(0xFF4F46E5), Color(0xFF4F46E5)),
    Ocean("Ocean", Color(0xFF0369A1), Color(0xFF0369A1)),
    Violet("Violet", Color(0xFF7C3AED), Color(0xFF7C3AED)),
    ;

    companion object {
        val Default = Graphite
    }
}

/** [palette]'s M3 scheme on neutral grey surfaces; [pureBlack] for OLED. */
fun appColorScheme(dark: Boolean, palette: Palette = Palette.Default, pureBlack: Boolean = false): ColorScheme = schemeOf(palette, dark, dark && pureBlack)

/** The [i]th [Tone] in [hex], a run of RRGGBB triples. */
private fun toneAt(hex: String, i: Int): Tone {
    fun c(j: Int) = Color(0xFF000000 or hex.substring(j * 6, j * 6 + 6).toLong(16))
    return Tone(c(i * 3), c(i * 3 + 1), c(i * 3 + 2))
}

/** [argb]'s CAM16 hue in degrees, as material-color-utilities' `Cam16.fromInt` (default viewing conditions). */
private fun cam16Hue(argb: Int): Double {
    fun lin(c: Int): Double = (c / 255.0).let { if (it <= 0.040449936) it / 12.92 else ((it + 0.055) / 1.055).pow(2.4) } * 100
    val r = lin(argb shr 16 and 0xFF)
    val g = lin(argb shr 8 and 0xFF)
    val b = lin(argb and 0xFF)
    val x = 0.41233895 * r + 0.35762064 * g + 0.18051042 * b
    val y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    val z = 0.01932141 * r + 0.11916382 * g + 0.95034478 * b
    fun adapt(c: Double, d: Double): Double {
        val f = (CAM16_FL * abs(c * d) / 100).pow(0.42)
        return Math.signum(c * d) * 400 * f / (f + 27.13)
    }
    val ra = adapt(0.401288 * x + 0.650173 * y - 0.051461 * z, CAM16_RGB_D[0])
    val ga = adapt(-0.250268 * x + 1.204414 * y + 0.045854 * z, CAM16_RGB_D[1])
    val ba = adapt(-0.002079 * x + 0.048952 * y + 0.953127 * z, CAM16_RGB_D[2])
    return (Math.toDegrees(atan2((ra + ga - 2 * ba) / 9, (11 * ra - 12 * ga + ba) / 11)) + 360) % 360
}

/**
 * Status seed [seed] (success, warning, danger, lab) harmonized toward [accentHue] as Material's
 * `Blend.harmonize`: turned up to 15° toward it, to the nearest degree.
 */
private fun turnedTone(seed: Int, accentHue: Double, dark: Boolean): Tone {
    val hue = SEED_HUES[seed]
    val diff = 180 - abs(abs(hue - accentHue) - 180)
    val direction = if ((accentHue - hue + 360) % 360 <= 180) 1 else -1
    val turn = (minOf(diff * 0.5, 15.0) * direction).roundToInt()
    return toneAt(if (dark) TURNED_TONES_DARK else TURNED_TONES_LIGHT, seed * 31 + turn + 15)
}

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
        /**
         * [palette]'s tones on [s]. The accent tone sits a step quieter than M3's container tone
         * (95 light, 25 dark at low chroma) so tints read as a hint on neutral surfaces, like the
         * Flutter app's. Status colours keep their meaning but lean toward the accent's hue, so
         * they belong to whichever palette (or wallpaper) is chosen.
         */
        fun of(s: ColorScheme, dark: Boolean, palette: Palette = Palette.Default): AppPalette {
            val hex = PALETTE_TONES.getValue(palette)[if (dark) 1 else 0]
            return of(s, dark, toneAt(hex, 0), List(4) { toneAt(hex, it + 1) })
        }

        /** The wallpaper's tones on [s] (Android 12+): the accent from the system palette, the status tones turned toward it. */
        @RequiresApi(31)
        fun wallpaper(s: ColorScheme, dark: Boolean, context: Context): AppPalette {
            fun c(id: Int) = Color(context.getColor(id))
            val accent = if (!dark) Tone(c(android.R.color.system_accent1_600), c(android.R.color.system_accent1_50), c(android.R.color.system_accent1_800))
            else Tone(c(android.R.color.system_accent1_200), lerp(c(android.R.color.system_accent2_700), c(android.R.color.system_accent2_800), 0.5f), c(android.R.color.system_accent1_100))
            val hue = cam16Hue(s.primary.toArgb())
            return of(s, dark, accent, List(4) { turnedTone(it, hue, dark) })
        }

        private fun of(s: ColorScheme, dark: Boolean, accent: Tone, status: List<Tone>) = AppPalette(
            accent = accent.base,
            onAccent = if (dark) s.surface else s.onPrimary,
            accentTone = accent,
            success = status[0],
            warning = status[1],
            danger = status[2],
            lab = status[3],
            shadow = if (dark) Color.Transparent else s.scrim.copy(alpha = 0.06f),
        )
    }
}

/** New name for the status tones; same object as [AppPalette]. */
typealias Tones = AppPalette
