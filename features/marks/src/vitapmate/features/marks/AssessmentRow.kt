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
import androidx.compose.foundation.layout.height
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
import vitapmate.ui.neutralTone
import vitapmate.ui.nums

/**
 * One assessment: what it adds to the total (its band's square without a weightage), title and chips, scored / max; a small bar of the score
 * with weightage → what it adds to the total; VTOP's remark.
 */
@Composable
internal fun AssessmentRow(assessment: Assessment, latest: Boolean, insights: Boolean = true, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tone = when {
        insights -> assessment.band.tone()
        assessment.band == ScoreBand.ABSENT -> AppTheme.tones.danger
        else -> neutralTone
    }
    val weight = assessment.weightage
    val weightedValue = if (insights) assessment.weighted else assessment.reportedWeighted
    Column(
        modifier.fillMaxWidth().padding(vertical = Space.md).semantics {
            contentDescription = listOfNotNull(
                assessment.title,
                assessment.scoreLabel,
                weight?.let { "weightage ${it.marks()}${weightedValue?.let { w -> ", weighted ${w.marks()}" }.orEmpty()}" },
                assessment.remark,
            ).joinToString(", ")
        },
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (insights) {
                // What it adds to the total, in its band's tone; the band's square without a weightage.
                if (weight != null) {
                    Box(Modifier.widthIn(min = 48.dp).height(30.dp).background(tone.subtle, RoundedCornerShape(10.dp)).padding(horizontal = 6.dp), Alignment.Center) {
                        Text(weightedValue?.let { "+${it.marks()}" } ?: "–", style = MaterialTheme.typography.labelMedium.nums(), color = tone.onSubtle, maxLines = 1)
                    }
                } else {
                    Box(Modifier.size(10.dp).background(tone.base, RoundedCornerShape(3.dp)))
                }
                Spacer(Modifier.width(Space.sm))
            }
            Text(
                assessment.title,
                Modifier.weight(1f),
                style = MaterialTheme.typography.titleSmall,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            if (insights && latest) {
                Spacer(Modifier.width(Space.sm))
                StatusChip("Latest", AppTheme.tones.accentTone)
            }
            if (insights && assessment.reevaluation) {
                Spacer(Modifier.width(Space.sm))
                StatusChip("Re-evaluation", ScoreBand.NOT_POSTED.tone())
            }
            Spacer(Modifier.width(Space.sm))
            val scored = assessment.scored
            if (scored != null && assessment.band != ScoreBand.ABSENT) {
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
        if (insights) {
            if (assessment.scored != null || weight != null) {
                Row(Modifier.padding(top = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                    ProgressBar(((assessment.percent ?: 0.0) / 100).toFloat(), tone.base, Modifier.weight(1f), height = 4.dp)
                    weight?.let {
                        Spacer(Modifier.width(Space.md))
                        Text(
                            "${it.marks()}% → ${weightedValue?.marks() ?: "–"}",
                            // A fixed width, so every row's bar ends at the same place.
                            Modifier.widthIn(min = 84.dp),
                            textAlign = TextAlign.End,
                            style = MaterialTheme.typography.labelMedium.nums(),
                            color = scheme.onSurfaceVariant,
                        )
                    }
                }
            }
        } else if (weight != null || weightedValue != null) {
            val detail = listOfNotNull(
                weight?.let { "Weightage ${it.marks()}%" },
                weightedValue?.let { "Weighted ${it.marks()}" },
            ).joinToString(" · ")
            Text(
                detail,
                Modifier.padding(top = Space.xs),
                style = MaterialTheme.typography.labelMedium.nums(),
                color = scheme.onSurfaceVariant,
            )
        }
        assessment.remark?.let { remark ->
            Text(
                remark,
                Modifier.padding(top = Space.xs),
                style = MaterialTheme.typography.labelMedium,
                color = scheme.onSurfaceVariant,
            )
        }
    }
}
