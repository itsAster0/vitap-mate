package vitapmate.features.calendar

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerDefaults
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronLeft
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import vitapmate.core.CalendarDay
import vitapmate.core.CalendarMonth
import vitapmate.ui.Space
import vitapmate.ui.Surface
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * The month grid in a card: the month's name beside a connected previous / next pair ([onStep] -1 / +1),
 * the months in a pager that springs to rest, a line for the [selected] day, and the legend.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun MonthCard(
    months: List<CalendarMonth>,
    pager: PagerState,
    today: Int,
    selected: Int?,
    onDay: (CalendarDay) -> Unit,
    onStep: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val motion = MaterialTheme.motionScheme
    val scheme = MaterialTheme.colorScheme
    val page = pager.currentPage.coerceIn(months.indices)
    val month = months[page]
    Surface(modifier.fillMaxWidth(), padding = PaddingValues(start = Space.sm, top = Space.sm, end = Space.sm, bottom = Space.md)) {
        Column {
            Row(Modifier.padding(start = Space.xs), verticalAlignment = Alignment.CenterVertically) {
                AnimatedContent(
                    page,
                    Modifier.weight(1f),
                    transitionSpec = {
                        val dir = if (targetState > initialState) 1 else -1
                        val slide = motion.fastSpatialSpec<IntOffset>()
                        (slideInHorizontally(slide) { it / 4 * dir } + fadeIn(motion.fastEffectsSpec())) togetherWith
                            (slideOutHorizontally(slide) { -it / 4 * dir } + fadeOut(motion.fastEffectsSpec()))
                    },
                    label = "month-title",
                ) { i ->
                    val first = months[i].days.first().day
                    Text(
                        buildAnnotatedString {
                            append(formatDay(first, "MMMM"))
                            withStyle(SpanStyle(color = scheme.onSurfaceVariant, fontWeight = FontWeight.Medium)) { append(" ${formatDay(first, "yyyy")}") }
                        },
                        Modifier.fillMaxWidth(),
                        style = MaterialTheme.typography.headlineSmallEmphasized.nums(),
                        color = scheme.onSurface,
                    )
                }
                val colors = IconButtonDefaults.filledTonalIconButtonColors(containerColor = scheme.surfaceContainerHigh, contentColor = scheme.onSurface)
                Row(horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                    FilledTonalIconButton(
                        { onStep(-1) },
                        Modifier.size(40.dp).testTag("calendar.prev"),
                        enabled = page > 0,
                        shape = RoundedCornerShape(20.dp, 8.dp, 8.dp, 20.dp),
                        colors = colors,
                    ) { Icon(Lucide.ChevronLeft, "Previous month", Modifier.size(20.dp)) }
                    FilledTonalIconButton(
                        { onStep(1) },
                        Modifier.size(40.dp).testTag("calendar.next"),
                        enabled = page < months.lastIndex,
                        shape = RoundedCornerShape(8.dp, 20.dp, 20.dp, 8.dp),
                        colors = colors,
                    ) { Icon(Lucide.ChevronRight, "Next month", Modifier.size(20.dp)) }
                }
            }
            Spacer(Modifier.height(Space.md))
            WeekdayHeader()
            // The card grows or shrinks to the shown month's weeks; a taller neighbour is clipped mid-swipe.
            val height by animateDpAsState(CELL_HEIGHT * weeksIn(month), motion.defaultSpatialSpec(), label = "grid-height")
            HorizontalPager(
                pager,
                Modifier.fillMaxWidth().height(height).clipToBounds(),
                verticalAlignment = Alignment.Top,
                flingBehavior = PagerDefaults.flingBehavior(pager, snapAnimationSpec = motion.defaultSpatialSpec()),
                key = { months[it].month },
            ) { i ->
                MonthGrid(months[i], today, selected, onDay, Modifier.testTag("calendar.month.${months[i].month}"))
            }
            val day = month.days.firstOrNull { it.day == selected }
            AnimatedVisibility(day != null, enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(), exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut()) {
                day?.let { DayLine(it, today, Modifier.padding(top = Space.sm)) }
            }
            Spacer(Modifier.height(Space.md))
            Legend()
        }
    }
}
