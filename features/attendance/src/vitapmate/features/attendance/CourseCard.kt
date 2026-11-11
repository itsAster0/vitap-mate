package vitapmate.features.attendance

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toPath
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.center
import androidx.compose.ui.graphics.Matrix
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.graphics.shapes.Morph
import com.composables.icons.lucide.Ban
import com.composables.icons.lucide.Lucide
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.ui.AppTheme
import vitapmate.ui.KindChip
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * One course: VTOP's percentage in a badge shaped by its standing, beside the name (two lines at
 * most), kind and code; the progress line (tick at 75%) in the standing's tone, attended/held
 * with the advice pill, any debarment in a banner, then the [plan]'s outlook cells.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseCard(
    course: AttendanceCourse,
    plan: AttendancePlan?,
    showSkipAdvice: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val standing = course.standing
    val tone = standing.tone()
    val danger = course.debarred
    // The chip is what VTOP's count allows right now; the outlook counts till the exam or the end.
    val advice = course.advice(showSkipAdvice)
    val name = course.name.ifBlank { course.code }
    val (attended, held) = course.sessions
    val debarLine = course.debarLine()
    SectionCard(
        modifier.fillMaxWidth()
            .tappable(onClick, name, Role.Button, pressedScale = 0.98f)
            .testTag("attendance.course.${course.key}")
            .semantics {
                contentDescription = listOfNotNull(
                    name,
                    if (course.total == 0) "no classes yet" else "${course.percentLabel}, $attended of $held attended",
                    debarLine,
                    advice,
                ).joinToString(", ")
            },
        shape = MaterialTheme.shapes.extraLargeIncreased,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            StandingBadge(standing, course.percentLabel, tone)
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(Space.xs))
                Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
                    KindChip(course.isLab)
                    Text(course.code, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant, maxLines = 1)
                }
            }
        }
        AttendanceLine(course.percent, tone.base, Modifier.padding(top = Space.md, bottom = Space.sm))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                if (course.total == 0) AnnotatedString("No classes yet")
                else buildAnnotatedString {
                    withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = scheme.onSurface)) { append("$attended") }
                    append("/$held${if (course.isLab) " sessions" else ""}")
                },
                Modifier.weight(1f),
                style = MaterialTheme.typography.bodyMedium.nums(),
                color = scheme.onSurfaceVariant,
            )
            if (danger) AdvicePill("Debarred", tone, solid = true)
            else advice?.let { AdvicePill(it, tone) }
        }
        debarLine?.let {
            Row(
                Modifier.padding(top = Space.md).fillMaxWidth().background(tone.subtle, RoundedCornerShape(14.dp)).padding(horizontal = Space.md, vertical = 9.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(Lucide.Ban, null, Modifier.size(16.dp), tint = tone.base)
                Spacer(Modifier.width(Space.sm))
                Text(it, style = MaterialTheme.typography.labelLarge.nums(), color = tone.onSubtle, maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
        }
        plan?.let { OutlookLine(course, it, showSkipAdvice, tone, Modifier.padding(top = Space.md)) }
    }
}

/** The advice as a pill on the tone's quiet fill; [solid] (debarred) on its base, with a ban icon. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun AdvicePill(label: String, tone: Tone, solid: Boolean = false) {
    val ink = if (solid) AppTheme.colors.card else tone.onSubtle
    Row(
        Modifier.background(if (solid) tone.base else tone.subtle, CircleShape).padding(horizontal = Space.md, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (solid) {
            Icon(Lucide.Ban, null, Modifier.size(14.dp), tint = ink)
            Spacer(Modifier.width(Space.xs))
        }
        Text(label, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/**
 * The percentage in a 62 dp Material shape that sharpens as the course gets into trouble: a soft
 * cookie when safe, a sun on the line, a burst below 75% (filled solid when debarred), a circle
 * before the first class. A new standing morphs the shape and fades the colours.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun StandingBadge(standing: Standing, label: String, tone: Tone) {
    val polygon = when (standing) {
        Standing.Safe -> MaterialShapes.Cookie9Sided
        Standing.Edge -> MaterialShapes.Sunny
        Standing.Below, Standing.Debarred -> MaterialShapes.Burst
        Standing.NoClasses -> MaterialShapes.Circle
    }
    var morph by remember { mutableStateOf(Morph(polygon, polygon)) }
    var shown by remember { mutableStateOf(polygon) }
    val progress = remember { Animatable(1f) }
    val spec = MaterialTheme.motionScheme.slowSpatialSpec<Float>()
    LaunchedEffect(polygon) {
        if (polygon == shown) return@LaunchedEffect
        morph = Morph(shown, polygon)
        shown = polygon
        progress.snapTo(0f)
        progress.animateTo(1f, spec)
    }
    val solid = standing == Standing.Debarred
    val fill by animateColorAsState(if (solid) tone.base else tone.subtle, MaterialTheme.motionScheme.slowEffectsSpec())
    val ink by animateColorAsState(if (solid) AppTheme.colors.card else tone.onSubtle, MaterialTheme.motionScheme.slowEffectsSpec())
    Box(
        Modifier.size(62.dp).drawWithCache {
            val path = Path()
            val scale = Matrix().apply { scale(size.width, size.height) }
            onDrawBehind {
                morph.toPath(progress.value.coerceIn(0f, 1f), path)
                path.transform(scale)
                path.translate(size.center - path.getBounds().center)
                drawPath(path, fill)
            }
        },
        Alignment.Center,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.titleLargeEmphasized.nums(fontSize = if (label.length > 3) 16.sp else 19.sp, fontWeight = FontWeight.W800, letterSpacing = (-0.5).sp),
            color = ink,
            maxLines = 1,
        )
    }
}
