package vitapmate.ui

import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import design.generated.resources.Res
import design.generated.resources.inter
import kotlin.math.exp
import org.jetbrains.compose.resources.Font

/** Inter (variable), one face per weight. */
@Composable
fun rememberInter(): FontFamily {
    val fonts = (100..900 step 100).map {
        Font(Res.font.inter, FontWeight(it), variationSettings = FontVariation.Settings(FontVariation.weight(it)))
    }
    return remember(fonts) { FontFamily(fonts) }
}

/**
 * Inter at [size] sp with Inter's "dynamic metrics" tracking, so headings sit tight and captions
 * keep a little air; [height] is a multiple of the size, centred.
 */
private fun tracked(size: Float, height: Float, family: FontFamily, weight: FontWeight? = null): TextStyle {
    val em = -0.0223f + 0.185f * exp(-0.1745f * size)
    return TextStyle(
        fontFamily = family,
        fontSize = size.sp,
        fontWeight = weight,
        lineHeight = height.em,
        letterSpacing = (em * size).sp,
        lineHeightStyle = LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.None),
    )
}

/**
 * The M3 type scale in Inter at the Flutter app's sizes (forui touch: xs 14, sm 16, md 18, lg 20,
 * xl 22, xl2 30, xl3 36) with M3-like line heights; emphasized styles one or two weights up.
 */
internal fun appTypography(f: FontFamily): Typography {
    fun t(size: Float, height: Float, weight: Int) = tracked(size, height, f, FontWeight(weight))
    return Typography(
        displayLarge = t(48f, 1.15f, 600), displayMedium = t(36f, 1.2f, 600), displaySmall = t(30f, 1.2f, 600),
        headlineLarge = t(30f, 1.25f, 600), headlineMedium = t(26f, 1.25f, 600), headlineSmall = t(22f, 1.3f, 600),
        titleLarge = t(20f, 1.35f, 600), titleMedium = t(18f, 1.4f, 500), titleSmall = t(16f, 1.4f, 500),
        bodyLarge = t(16f, 1.5f, 400), bodyMedium = t(14f, 1.45f, 400), bodySmall = t(12f, 1.35f, 400),
        labelLarge = t(14f, 1.4f, 500), labelMedium = t(12f, 1.35f, 500), labelSmall = t(11f, 1.3f, 500),
        displayLargeEmphasized = t(48f, 1.15f, 800), displayMediumEmphasized = t(36f, 1.2f, 800),
        displaySmallEmphasized = t(30f, 1.2f, 800), headlineLargeEmphasized = t(30f, 1.25f, 700),
        headlineMediumEmphasized = t(26f, 1.25f, 700), headlineSmallEmphasized = t(22f, 1.3f, 700),
        titleLargeEmphasized = t(20f, 1.35f, 700), titleMediumEmphasized = t(18f, 1.4f, 700),
        titleSmallEmphasized = t(16f, 1.4f, 600), bodyLargeEmphasized = t(16f, 1.5f, 600),
        bodyMediumEmphasized = t(14f, 1.45f, 600), bodySmallEmphasized = t(12f, 1.35f, 600),
        labelLargeEmphasized = t(14f, 1.4f, 700), labelMediumEmphasized = t(12f, 1.35f, 700),
        labelSmallEmphasized = t(11f, 1.3f, 700),
    )
}

/** forui's `FTypeface` sizes (touch): xs3 10 … xl8 108. New code uses `MaterialTheme.typography`. */
@Immutable
class AppType(private val family: FontFamily = FontFamily.Default) {
    val xs3: TextStyle = tracked(10f, 1f, family)
    val xs2: TextStyle = tracked(12f, 1f, family)
    val xs: TextStyle = tracked(14f, 1.25f, family)
    val sm: TextStyle = tracked(16f, 1.5f, family)
    val md: TextStyle = tracked(18f, 1.75f, family)
    val lg: TextStyle = tracked(20f, 1.75f, family)
    val xl: TextStyle = tracked(22f, 2f, family)
    val xl2: TextStyle = tracked(30f, 2.25f, family)
    val xl3: TextStyle = tracked(36f, 2.5f, family)
    val xl4: TextStyle = tracked(48f, 1f, family)
    val xl5: TextStyle = tracked(60f, 1f, family)
    val xl6: TextStyle = tracked(72f, 1f, family)
    val xl7: TextStyle = tracked(96f, 1f, family)
    val xl8: TextStyle = tracked(108f, 1f, family)
}
