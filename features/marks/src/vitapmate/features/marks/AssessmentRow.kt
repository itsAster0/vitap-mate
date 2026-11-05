package vitapmate.features.marks

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import vitapmate.core.Assessment
import vitapmate.core.ScoreBand
import vitapmate.ui.AppTheme
import vitapmate.ui.ProgressBar
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.nums

/**
 * One assessment: its band's square, title and chips, scored / max; a small bar of the score
 * with weightage → what it adds to the total; VTOP's remark.
 */
@Composable
internal fun AssessmentRow(assessment: Assessment, latest: Boolean, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tone = assessment.band.tone()
    val weight = assessment.weightage
    Column(
        modifier.fillMaxWidth().padding(vertical = Space.md).semantics {
            contentDescription = listOfNotNull(
                assessment.title,
                assessment.scoreLabel,
                weight?.let { "weightage ${it.marks()}, adds ${(assessment.weighted ?: 0.0).marks()}" },
                assessment.remark.ifBlank { null },
            ).joinToString(", ")
        },
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(10.dp).background(tone.base, RoundedCornerShape(3.dp)))
            Spacer(Modifier.width(Space.sm))
            Text(
                assessment.title,
                Modifier.weight(1f),
                style = MaterialTheme.typography.titleSmall,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            if (latest) {
                Spacer(Modifier.width(Space.sm))
                StatusChip("Latest", AppTheme.tones.accentTone)
            }
            if (assessment.reevaluation) {
                Spacer(Modifier.width(Space.sm))
                StatusChip("Re-evaluation", ScoreBand.NOT_POSTED.tone())
            }
            Spacer(Modifier.width(Space.sm))
            val scored = assessment.scored
            if (scored != null) {
                Text(
                    buildAnnotatedString {
                        withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(scored.marks()) }
                        withStyle(SpanStyle(color = scheme.onSurfaceVariant)) { append(" / ${assessment.max.marks()}") }
                    },
                    style = MaterialTheme.typography.titleSmall.nums(),
                )
            } else {
                StatusChip(assessment.scoreLabel, tone)
            }
        }
        if (assessment.scored != null || weight != null) {
            Row(Modifier.padding(top = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                ProgressBar(((assessment.percent ?: 0.0) / 100).toFloat(), tone.base, Modifier.weight(1f), height = 4.dp)
                weight?.let {
                    Spacer(Modifier.width(Space.md))
                    Text(
                        "${it.marks()}% → ${assessment.weighted?.marks() ?: "–"}",
                        // A fixed width, so every row's bar ends at the same place.
                        Modifier.widthIn(min = 84.dp),
                        textAlign = TextAlign.End,
                        style = MaterialTheme.typography.labelMedium.nums(),
                        color = scheme.onSurfaceVariant,
                    )
                }
            }
        }
        if (assessment.remark.isNotBlank()) {
            Text(
                assessment.remark,
                Modifier.padding(top = Space.xs),
                style = MaterialTheme.typography.labelMedium,
                color = scheme.onSurfaceVariant,
            )
        }
    }
}
