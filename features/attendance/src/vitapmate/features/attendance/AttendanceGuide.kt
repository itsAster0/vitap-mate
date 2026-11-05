package vitapmate.features.attendance

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Ban
import com.composables.icons.lucide.CalendarClock
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.Repeat
import com.composables.icons.lucide.TriangleAlert
import vitapmate.ui.AppTheme
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.Tone

/** The attendance feature's detail item for [AttendanceGuide]. */
const val ATTENDANCE_GUIDE = "guide"

/** The disclaimer shown before "Classes you can skip" turns on. */
const val SKIP_DISCLAIMER =
    "These are estimates from VTOP's attendance, your timetable and the academic calendar. " +
        "They can be wrong, for example when a day follows another day's timetable or a holiday " +
        "is announced late. Check VTOP before you skip a class. The app isn't responsible for " +
        "attendance you lose by relying on these numbers."

/**
 * Settings → How the numbers work (the Flutter app's attendance guide): what each number on a
 * course means, worked through one example course, and when the estimate can be off.
 */
@Composable
fun AttendanceGuide() {
    val tones = AppTheme.tones
    val bottom = LocalContentPadding.current.calculateBottomPadding()
    LazyColumn(
        Modifier.testTag("attendance.guide"),
        contentPadding = PaddingValues(start = Space.page, end = Space.page, top = Space.sm, bottom = bottom + Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        item("warning") {
            Row(
                Modifier.fillMaxWidth().background(tones.warning.subtle, MaterialTheme.shapes.extraLarge).padding(Space.lg),
                horizontalArrangement = Arrangement.spacedBy(Space.md),
            ) {
                Icon(Lucide.TriangleAlert, null, Modifier.size(22.dp), tint = tones.warning.onSubtle)
                Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    Text("Best effort, not official", style = MaterialTheme.typography.titleMedium, color = tones.warning.onSubtle)
                    Text(
                        "The app works these numbers out from VTOP's attendance, your timetable and the academic " +
                            "calendar. They can be wrong. Check VTOP before you skip a class. The app isn't " +
                            "responsible for attendance you lose by relying on them.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = tones.warning.onSubtle,
                    )
                }
            }
        }
        item("example") { ExampleCourse() }
        item("numbers") {
            SectionCard(title = "What each number means") {
                Column(verticalArrangement = Arrangement.spacedBy(Space.lg)) {
                    Explained(1, "92%", "VTOP's own percentage. VTOP rounds up, so 74.5% shows as 75%. The app checks 75% against the exact figure, so a course showing 75% can still say Attend 1.")
                    Explained(2, "22/24", "Classes VTOP has posted, attended out of held. On duty counts as attended.")
                    Explained(
                        3,
                        "Can skip 5",
                        "Classes you can miss in a row and stay at 75% or above. Below 75% it reads Attend 2, the classes in a row you need to get back. Don't skip means the next miss takes you below 75%.",
                        math = "22 ÷ 29 = 75.9%     22 ÷ 30 = 73.3%",
                    ) { SkipStrip() }
                    Explained(4, "1+", "Classes already held that VTOP hasn't posted yet. The app can't know whether you went, so it still counts them as open.")
                    Explained(5, "16 left", "Classes still to come, from your timetable and the academic calendar. Holidays, exam days and Sundays don't count. Today's classes count until they start.")
                    Explained(
                        6,
                        "8 to spare",
                        "How many of the 17 open classes you can miss and still finish at 75% or above. Below 75% it reads 3 needed. Short of 75% means attending every open class won't get you there.",
                        math = "(22 + 9) ÷ (24 + 17) = 75.6%",
                    )
                    Explained(7, "95% max", "Where you finish if you attend every open class.", math = "(22 + 17) ÷ (24 + 17) = 95.1%")
                }
            }
        }
        item("labs") {
            SectionCard(title = "Labs") {
                Row(horizontalArrangement = Arrangement.spacedBy(Space.md)) {
                    Icon(Lucide.FlaskConical, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(
                        "VTOP counts each two-period lab session as 2 classes. The percentage and Can skip use VTOP's count. Left and to spare count whole sessions. Labs stop at the lab FAT.",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
        }
        item("off") {
            SectionCard(title = "Where it can be off") {
                Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
                    OffReason(Lucide.Repeat, "A day that follows another day's timetable, such as a Saturday running Monday's classes.")
                    OffReason(Lucide.CalendarX, "A holiday announced after the academic calendar, or one VTOP adds late.")
                    OffReason(Lucide.CalendarClock, "The college counting attendance only up to a cut-off date before an exam, not every class.")
                    OffReason(Lucide.Ban, "Cancelled, extra or rescheduled classes.")
                    OffReason(Lucide.Clock, "Attendance posted late, or corrected by the faculty afterwards.")
                    OffReason(Lucide.RefreshCw, "Data on your phone older than VTOP. Check Updated at the bottom of the page and refresh.")
                }
            }
        }
    }
}

/** One example course, drawn like a course card, with each number marked 1 to 7. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ExampleCourse() {
    val tones = AppTheme.tones
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    SectionCard(title = "An example course") {
        Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                Text("92%", style = MaterialTheme.typography.displaySmallEmphasized)
                Mark(1)
                Box(Modifier.weight(1f))
                Text("22/24", style = MaterialTheme.typography.titleMedium, color = muted)
                Mark(2)
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                Text(
                    "Can skip 5",
                    Modifier.background(tones.success.subtle, CircleShape).padding(horizontal = Space.md, vertical = Space.xs),
                    style = MaterialTheme.typography.labelLarge,
                    color = tones.success.onSubtle,
                )
                Mark(3)
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
                Text("1+", style = MaterialTheme.typography.bodyMedium, color = muted)
                Mark(4)
                Text("16 left ·", style = MaterialTheme.typography.bodyMedium)
                Mark(5)
                Text("8 to spare ·", style = MaterialTheme.typography.bodyMedium)
                Mark(6)
                Text("95% max", style = MaterialTheme.typography.bodyMedium)
                Mark(7)
            }
        }
    }
}

/** A small numbered dot tying a number on the example to its explanation. */
@Composable
private fun Mark(n: Int) {
    val scheme = MaterialTheme.colorScheme
    Box(Modifier.size(20.dp).background(scheme.primaryContainer, CircleShape), Alignment.Center) {
        Text("$n", style = MaterialTheme.typography.labelSmall, color = scheme.onPrimaryContainer)
    }
}

@Composable
private fun Explained(n: Int, title: String, body: String, math: String? = null, visual: (@Composable () -> Unit)? = null) {
    Row(horizontalArrangement = Arrangement.spacedBy(Space.md)) {
        Mark(n)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(body, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            math?.let {
                Text(
                    it,
                    Modifier.background(MaterialTheme.colorScheme.surfaceContainerHigh, MaterialTheme.shapes.medium).padding(horizontal = Space.md, vertical = Space.sm),
                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                )
            }
            visual?.invoke()
        }
    }
}

/** "Can skip 5" as dots: 22 attended, 2 missed, then 5 more misses that keep 75% and the one that doesn't. */
@Composable
private fun SkipStrip() {
    val tones = AppTheme.tones
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Row(horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
            repeat(8) { Dot(tones.success) }
            Text("…", style = MaterialTheme.typography.labelSmall)
            repeat(2) { Dot(tones.danger) }
            Box(Modifier.size(Space.xs))
            repeat(5) { Dot(tones.success, hollow = true) }
            Dot(tones.danger, hollow = true)
        }
        Text("Attended and missed so far, then 5 skips that keep 75%. The 6th doesn't.", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun Dot(tone: Tone, hollow: Boolean = false) {
    val m = Modifier.size(10.dp)
    Box(if (hollow) m.border(1.5.dp, tone.base, CircleShape) else m.background(tone.base, CircleShape))
}

@Composable
private fun OffReason(icon: ImageVector, text: String) {
    val scheme = MaterialTheme.colorScheme
    Row(horizontalArrangement = Arrangement.spacedBy(Space.md), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(36.dp).background(scheme.secondaryContainer, CircleShape), Alignment.Center) {
            Icon(icon, null, Modifier.size(18.dp), tint = scheme.onSecondaryContainer)
        }
        Text(text, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
    }
}
