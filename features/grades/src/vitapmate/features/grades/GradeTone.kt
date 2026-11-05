package vitapmate.features.grades

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import java.util.Locale
import vitapmate.core.ScoreBand
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/** S success, A lab, B accent, C/D warning, E/F/N danger, anything else neutral (the Flutter app's `gradeTone`). */
@Composable
internal fun gradeTone(grade: String): Tone = AppTheme.tones.let {
    when (grade.trim().uppercase()) {
        "S" -> it.success
        "A" -> it.lab
        "B" -> it.accentTone
        "C", "D" -> it.warning
        "E", "F", "N" -> it.danger
        else -> MaterialTheme.colorScheme.run { Tone(outline, surfaceContainerHigh, onSurfaceVariant) }
    }
}

/** Strong in success, fair in the accent, low in warning, else neutral. */
@Composable
internal fun ScoreBand.tone(): Tone = AppTheme.tones.let {
    when (this) {
        ScoreBand.STRONG -> it.success
        ScoreBand.FAIR -> it.accentTone
        ScoreBand.LOW -> it.warning
        ScoreBand.ABSENT -> it.danger
        ScoreBand.NOT_POSTED -> MaterialTheme.colorScheme.run { Tone(outline, surfaceContainerHigh, onSurfaceVariant) }
    }
}

/** 9.1234 → "9.12". */
internal fun Double.fixed2(): String = String.format(Locale.ROOT, "%.2f", this)

/** 4.0 → "4", 2.5 → "2.5". */
internal fun Double.creditsLabel(): String =
    if (this == Math.rint(this)) String.format(Locale.ROOT, "%.0f", this) else String.format(Locale.ROOT, "%.1f", this)
