package vitapmate.features.biometric

import vitapmate.data.istDay
import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import vitapmate.data.PageState
import vitapmate.data.Settings
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.DayPicker
import vitapmate.ui.PageScaffold
import vitapmate.ui.Radii
import vitapmate.ui.Skeleton
import vitapmate.ui.SkeletonList
import vitapmate.ui.Space
import vitapmate.ui.formatDay
import vitapmate.ui.rememberNow


/** The first day the picker offers (the Flutter app's calendar start). */
private const val FIRST_DAY = 18_262 // 2020-01-01

/**
 * Biometric history (`page.biometric`, log src `ui.biometric`): one day at a time, cached per
 * day in Rust. The last week as chips plus a calendar for older days (up to tomorrow), as the
 * Flutter app. Picking a day shows its saved copy; with Auto Refresh on, a day not over when it
 * was fetched is fetched again (flow `biometric.day`). Only counts are logged, never a time or
 * place.
 */
@Composable
fun BiometricScreen() {
    val now by rememberNow()
    val today = istDay(now)
    var day by rememberSaveable { mutableIntStateOf(today) }
    val page = rememberPage(
        "biometric",
        { core, flow -> core.cachedBiometric(flow, day) },
        { core, flow -> core.fetchBiometric(flow, day) },
        { it.updatedAt },
        item = "$day",
    ) { it.punches.size }
    val scope = rememberCoroutineScope()
    var picking by remember { mutableStateOf(false) }
    // The first day shown follows rememberPage's open rules; later picks refresh here.
    var firstPage by remember { mutableStateOf<Any?>(null) }
    LaunchedEffect(page) {
        if (firstPage == null) {
            firstPage = page
            return@LaunchedEffect
        }
        if (!Settings.autoRefresh) return@LaunchedEffect
        val s = snapshotFlow { page.state }.first { it !is PageState.Loading } as? PageState.Ready ?: return@LaunchedEffect
        // Over when fetched: VTOP adds nothing more. Just fetched (no copy saved): no second request.
        if (s.data.complete || System.currentTimeMillis() / 1000 - s.updatedAt < 60) return@LaunchedEffect
        page.refresh("pick", user = false)
    }
    fun select(target: Int, by: String) {
        if (target == day) return
        day = target
        scope.launch {
            flow("biometric.day") { id -> Log.info("ui.biometric", "day -${today - target}", id, mapOf("ago" to "${today - target}", "by" to by)) }
        }
    }
    LaunchedEffect(page.state, day, today) {
        val data = (page.state as? PageState.Ready)?.data
        StateRegistry.report(
            "ui.biometric",
            mapOf(
                "ago" to today - day,
                "punches" to data?.punches?.size,
                "entries" to data?.entries,
                "exits" to data?.exits,
                "complete" to data?.complete,
                "latest" to data?.punches?.firstOrNull()?.kind?.name,
            ),
        )
    }
    Column(Modifier.fillMaxSize()) {
        DateChips(today, day, onSelect = { select(it, "chip") }, onPick = { picking = true })
        PageScaffold(
            page.state,
            { page.refresh("pull") },
            isEmpty = { false },
            empty = {},
            modifier = Modifier.weight(1f),
            loading = {
                Column(Modifier.padding(Space.page)) {
                    Skeleton(Modifier.fillMaxWidth(), height = 72.dp, radius = Radii.lg)
                    Spacer(Modifier.height(Space.md))
                    SkeletonList(count = 3, height = 64.dp)
                }
            },
        ) { data -> DayView(data, isToday = day == today) }
    }
    if (picking) {
        DayPicker(day, FIRST_DAY..(today + 1), onDismiss = { picking = false }, modifier = Modifier.testTag("biometric.picker")) {
            picking = false
            select(it, "calendar")
        }
    }
}

/** Today, Yesterday, then five weekdays, then "Pick date" (selected for anything older). */
@Composable
private fun DateChips(today: Int, day: Int, onSelect: (Int) -> Unit, onPick: () -> Unit) {
    LazyRow(
        Modifier.testTag("biometric.days"),
        rememberLazyListState(),
        contentPadding = PaddingValues(horizontal = Space.page, vertical = Space.xs),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        items(7, key = { it }) { ago ->
            val haptics = rememberHaptics()
            val d = today - ago
            FilterChip(
                selected = d == day,
                onClick = { haptics.tick(); onSelect(d) },
                label = {
                    Text(
                        when (ago) {
                            0 -> "Today"
                            1 -> "Yesterday"
                            else -> formatDay(d, "EEE d")
                        },
                        maxLines = 1,
                    )
                },
                leadingIcon = if (d == day) ({ Icon(Lucide.Check, null, Modifier.size(FilterChipDefaults.IconSize)) }) else null,
                modifier = Modifier.testTag("biometric.day.$ago"),
            )
        }
        item("pick") {
            FilterChip(
                selected = today - day >= 7,
                onClick = onPick,
                label = { Text(if (today - day >= 7) formatDay(day, "d MMM yyyy") else "Pick date", maxLines = 1) },
                leadingIcon = { Icon(Lucide.CalendarDays, null, Modifier.size(FilterChipDefaults.IconSize)) },
                modifier = Modifier.testTag("biometric.pick"),
            )
        }
    }
}

