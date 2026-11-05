package vitapmate.features.marks

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import java.math.BigDecimal
import java.math.RoundingMode
import vitapmate.core.Assessment
import vitapmate.core.ClassKind
import vitapmate.core.MarksCourse
import vitapmate.core.ScoreBand
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/** Strong in success, fair in the accent, low in warning, absent in danger, not posted neutral. */
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

/** 12 → "12", 10.65 → "10.65", 9.5 → "9.5". */
internal fun Double.marks(): String =
    BigDecimal.valueOf(this).setScale(2, RoundingMode.HALF_UP).stripTrailingZeros().toPlainString()

/** "86%". */
internal fun Double.percentLabel(): String = "${BigDecimal.valueOf(this).setScale(0, RoundingMode.HALF_UP).toPlainString()}%"

/** "32 / 50", or "Not posted" / "Absent". */
internal val Assessment.scoreLabel: String
    get() = scored?.let { "${it.marks()} / ${max.marks()}" } ?: if (band == ScoreBand.ABSENT) "Absent" else "Not posted"

internal val MarksCourse.isLab get() = kind == ClassKind.LAB

/** Weightage of the course's 100 not assessed yet. */
internal val MarksCourse.toCome get() = (100.0 - maxWeighted).coerceAtLeast(0.0)
