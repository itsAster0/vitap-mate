package vitapmate.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

/** Spacing scale (4pt grid). */
object Space {
    val xs = 4.dp
    val sm = 8.dp
    val md = 12.dp
    val lg = 16.dp
    val xl = 24.dp
    val xxl = 32.dp
    /** Horizontal inset for page content inside the shell. */
    val page = 16.dp
}

/** Corner radii used across surfaces; the M3 shape scale is built from them. */
object Radii {
    val sm = 8.dp
    val md = 12.dp
    val lg = 16.dp
    val xl = 24.dp
    val pill = 999.dp
}

/** Motion durations in ms for hand-written tweens; M3 components use the expressive motion scheme. */
object Motion {
    const val FAST = 120
    const val MEDIUM = 220
    const val SLOW = 360
}

private val AppShapes = Shapes(
    extraSmall = RoundedCornerShape(6.dp),
    small = RoundedCornerShape(Radii.sm),
    medium = RoundedCornerShape(Radii.md),
    large = RoundedCornerShape(Radii.lg),
    largeIncreased = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(Radii.xl),
    extraLargeIncreased = RoundedCornerShape(28.dp),
    extraExtraLarge = RoundedCornerShape(36.dp),
)

private val LocalColors = staticCompositionLocalOf { AppColors.of(appColorScheme(false), false) }
private val LocalPalette = staticCompositionLocalOf { AppPalette.of(appColorScheme(false), false) }
private val LocalType = staticCompositionLocalOf { AppType() }

/** The app-wide snackbar queue (see [ToastHost]); screens may show their own snackbars on it. */
val LocalSnackbar = staticCompositionLocalOf { SnackbarHostState() }

object AppTheme {
    /** Status tones (success, warning, danger, lab, accent) on top of the M3 scheme. */
    val tones: Tones @Composable @ReadOnlyComposable get() = LocalPalette.current
    /** Older name for [tones]. */
    val palette: AppPalette @Composable @ReadOnlyComposable get() = LocalPalette.current
    /** forui's neutral roles, derived from the M3 scheme (older components). */
    val colors: AppColors @Composable @ReadOnlyComposable get() = LocalColors.current
    /** forui's sizes (older components); new code uses `MaterialTheme.typography`. */
    val type: AppType @Composable @ReadOnlyComposable get() = LocalType.current
}

/**
 * Material 3 Expressive with Inter and [palette]'s colours; null [palette] takes the wallpaper's
 * (Android 12+, else the default palette). Status tones lean toward the accent. [pureBlack]
 * makes dark surfaces black (OLED).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AppTheme(dark: Boolean = isSystemInDarkTheme(), palette: Palette? = Palette.Default, pureBlack: Boolean = false, content: @Composable () -> Unit) {
    val context = LocalContext.current
    val wallpaper = palette == null && Build.VERSION.SDK_INT >= 31
    val chosen = palette ?: Palette.Default
    val scheme = when {
        wallpaper -> if (!dark) dynamicLightColorScheme(context)
        else dynamicDarkColorScheme(context).let { if (pureBlack) it.copy(background = Color.Black, surface = Color.Black, surfaceContainerLowest = Color.Black) else it }
        else -> remember(dark, chosen, pureBlack) { appColorScheme(dark, chosen, pureBlack) }
    }
    val accent = if (wallpaper) scheme.primary else chosen.accent
    val inter = rememberInter()
    val type = remember(inter) { AppType(inter) }
    val typography = remember(inter) { appTypography(inter) }
    val snackbar = remember { SnackbarHostState() }
    MaterialExpressiveTheme(scheme, MotionScheme.expressive(), AppShapes, typography) {
        CompositionLocalProvider(
            LocalColors provides remember(scheme, dark) { AppColors.of(scheme, dark) },
            LocalPalette provides remember(scheme, dark, accent) { AppPalette.of(scheme, dark, accent) },
            LocalType provides type,
            LocalSnackbar provides snackbar,
            LocalContentColor provides scheme.onBackground,
            LocalTextStyle provides MaterialTheme.typography.bodyLarge,
            content = content,
        )
    }
}
