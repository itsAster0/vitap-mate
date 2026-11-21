package vitapmate.features.wrapped

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.work.WorkInfo
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Share
import com.composables.icons.lucide.Sparkles
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.min
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.Core
import vitapmate.core.WrappedReport
import vitapmate.data.AppCore
import vitapmate.data.BiometricFetch
import vitapmate.data.BiometricFetchWorker
import vitapmate.data.Problem
import vitapmate.data.ProblemKind
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.Tunable
import vitapmate.data.askedOtp
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.EmptyView
import vitapmate.ui.ProblemView
import vitapmate.ui.ToastTone
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** The design frame's size (the mockups' phone); slides are laid out in it and scaled. */
internal const val DESIGN_WIDTH = 360f
private const val DESIGN_HEIGHT = 640f

/** What the story draws from: the report, the class days still without campus times, the first name. */
private class Loaded(val report: WrappedReport?, val missing: List<Int>, val name: String?)

/**
 * Wrapped for the selected semester (flow `wrapped.open`, log src `ui.wrapped`): the cached
 * report at once, then what's missing is fetched one page at a time and the report read again.
 * "Putting your semester together…" shows only while there is nothing to draw. Switching
 * semester starts over. Opening marks the semester seen ([Settings.wrappedSeen]).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun WrappedScreen() {
    val context = LocalContext.current
    val semester = SemesterChoice.selected
    var attempt by remember { mutableIntStateOf(0) }
    var loaded by remember(semester) { mutableStateOf<Loaded?>(null) }
    var loading by remember(semester, attempt) { mutableStateOf(true) }
    var problem by remember(semester, attempt) { mutableStateOf<Problem?>(null) }
    LaunchedEffect(semester, attempt) {
        if (semester == null) {
            loading = false
            return@LaunchedEffect
        }
        if (semester !in Settings.wrappedSeen) Settings.wrappedSeen += semester
        flow("wrapped.open") { f ->
            /** Reads the cache again; a failure (a Rust panic) keeps the story on screen, else shows Unexpected. */
            suspend fun reread(): Boolean = try {
                loaded = read(f)
                true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                Log.warn("ui.wrapped", "read failed", f, mapOf("error" to (e::class.simpleName ?: "Throwable"), "kept" to "${loaded?.report != null}"))
                if (loaded?.report == null) problem = problemOf(e, f)
                false
            }
            if (!reread()) return@flow
            problem = fetchMissing(f, semester, loaded?.report)
            reread()
            Log.info("ui.wrapped", "open", f, mapOf("report" to "${loaded?.report != null}", "missing_days" to "${loaded?.missing?.size}", "problem" to "${problem?.kind}"))
        }
        loading = false
    }
    // Campus times: the worker's run for this semester; when it ends, the report is read again.
    val work by remember(semester) { semester?.let { BiometricFetch.state(context, it) } ?: flowOf(null) }.collectAsState(null)
    val running = work?.state.let { it == WorkInfo.State.ENQUEUED || it == WorkInfo.State.RUNNING || it == WorkInfo.State.BLOCKED }
    var sawRunning by remember(semester) { mutableStateOf(false) }
    LaunchedEffect(running) {
        if (running) sawRunning = true
        else if (sawRunning) {
            sawRunning = false
            flow("wrapped.reload") { f ->
                try {
                    loaded = read(f)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    Log.warn("ui.wrapped", "reload failed", f, mapOf("error" to (e::class.simpleName ?: "Throwable"), "kept" to "${loaded?.report != null}"))
                    if (loaded?.report == null) problem = problemOf(e, f)
                }
            }
        }
    }
    val report = loaded?.report
    when {
        semester != null && report != null -> {
            val scope = rememberCoroutineScope()
            val missing = loaded?.missing.orEmpty()
            // A hand-over starts a fresh run whose progress is empty until its first count: keep the last one.
            var shown by remember(semester) { mutableStateOf(0 to 0) }
            LaunchedEffect(work, running) {
                val p = work?.progress
                val total = p?.getInt(BiometricFetchWorker.TOTAL, 0) ?: 0
                if (!running) shown = 0 to 0 else if (p != null && total > 0) shown = p.getInt(BiometricFetchWorker.DONE, 0) to total
            }
            val campus = CampusFetch(
                missing,
                running,
                shown.first,
                shown.second.takeIf { it > 0 } ?: missing.size,
            ) {
                scope.logAction("wrapped.fetch", "ui.wrapped", "campus fetch", mapOf("days" to "${missing.size}"))
                BiometricFetch.start(context, semester, missing)
            }
            Story(report, semester, loaded?.name, campus)
        }
        loading -> Box(Modifier.fillMaxSize().background(Ink.Accent.tone().subtle), Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
                LoadingIndicator(Modifier.size(64.dp))
                Text("Putting your semester together…", style = MaterialTheme.typography.titleMediumEmphasized, color = Ink.Accent.tone().onSubtle)
            }
        }
        else -> Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surface).windowInsetsPadding(WindowInsets.systemBars), Alignment.Center) {
            val p = problem
            if (p != null) ProblemView(p, { attempt++ })
            else EmptyView(
                Lucide.Sparkles,
                "Nothing to wrap yet",
                Modifier.testTag("wrapped.empty"),
                message = "Wrapped opens once this semester's classes are over and its calendar is saved.",
            )
        }
    }
}

/** The report, missing campus days and first name from the cache, off the main thread. */
private suspend fun read(f: String): Loaded = withContext(Dispatchers.IO) {
    val core = AppCore.await()
    Loaded(core.cachedWrapped(f), core.wrappedMissingDays(f), core.cachedProfile(f)?.name?.let(::firstName))
}

/**
 * The given name from VTOP's "SURNAME GIVEN [OTHER]" order: "MALLIDI YASWANTH REDDY" as
 * "Yaswanth", a lone word as itself. Never the registration number.
 */
private fun firstName(name: String) = name.trim().split(Regex("\\s+")).filter { it.isNotBlank() }.let { it.getOrNull(1) ?: it.firstOrNull() }?.let(::titled)

/**
 * Fetches, one at a time, the pages the story reads that aren't cached (grades also while no
 * grade is posted, and the grade history while the report has no CGPA, when older than
 * [Tunable.WrappedRefetch]), then the last semester's attendance. A failure logs a
 * warning and the rest go on, unless it would fail them all too (offline, signed out, VTOP
 * down); the first one is returned.
 */
private suspend fun fetchMissing(f: String, semester: String, report: WrappedReport?): Problem? {
    val core = AppCore.await()
    var first: Problem? = null
    /** Runs [block] off the main thread; false when the rest should stop. */
    suspend fun step(page: String, block: suspend (Core) -> Unit): Boolean = try {
        withContext(Dispatchers.IO) { block(core) }
        true
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        val p = problemOf(e, f)
        Log.warn("ui.wrapped", "fetch failed, left out", f, mapOf("page" to page, "error" to (e::class.simpleName ?: "Throwable"), "kind" to p.kind.name))
        if (first == null) first = p
        askedOtp(e)
        p.kind == ProblemKind.Unexpected
    }
    suspend fun <T> cached(read: (Core) -> T): T = withContext(Dispatchers.IO) { read(core) }
    val maxAge = Settings.tunable(Tunable.WrappedRefetch) * 3600L
    /** Not cached, or cached while [missing] and older than [Tunable.WrappedRefetch]. */
    fun stale(updatedAt: Long?, missing: Boolean) = updatedAt == null || missing && System.currentTimeMillis() / 1000 - updatedAt > maxAge
    if (cached { it.cachedAttendance(f) } == null && !step("attendance") { it.fetchAttendance(f) }) return first
    val histories = cached { c -> c.cachedAttendance(f)?.courses?.map { it.key }.orEmpty().filter { c.cachedAttendanceDetail(f, it) == null } + c.mismatchedAttendanceDetails(f) }.distinct()
    for (key in histories) if (!step("history") { it.fetchAttendanceDetail(f, key) }) return first
    val pages = listOf<Triple<String, (Core) -> Boolean, suspend (Core) -> Unit>>(
        Triple("timetable", { it.cachedTimetable(f) == null }, { it.fetchTimetable(f) }),
        Triple("calendar", { it.cachedCalendar(f) == null }, { it.fetchCalendar(f) }),
        Triple("marks", { it.cachedMarks(f) == null }, { it.fetchMarks(f) }),
        Triple("grades", { stale(it.cachedGrades(f, semester)?.updatedAt, report?.grades == null) }, { it.fetchGrades(f, semester) }),
        Triple("grade_history", { stale(it.cachedGradeHistory(f)?.updatedAt, report?.cgpa == null) }, { it.fetchGradeHistory(f) }),
        Triple("exams", { it.cachedExams(f) == null }, { it.fetchExams(f) }),
        Triple("general_outing", { it.cachedGeneralOuting(f) == null }, { it.fetchGeneralOuting(f) }),
        Triple("weekend_outing", { it.cachedWeekendOuting(f) == null }, { it.fetchWeekendOuting(f) }),
        // Fetches only when the older semester's attendance isn't cached.
        Triple("previous", { true }, { it.fetchWrappedPrevious(f) }),
    )
    for ((page, needed, fetch) in pages) if (cached(needed) && !step(page, fetch)) return first
    return first
}

/**
 * The story: one slide at a time over its tone, a segmented progress bar, tap the left third to
 * go back and the rest to go on, swipe down to close. Share draws the slide as an image; Hide
 * grades (remembered per semester) changes only that image.
 */
@Composable
private fun Story(report: WrappedReport, semester: String, name: String?, campus: CampusFetch) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val slides = remember(report, campus.missing.isNotEmpty()) { slides(report, campus.missing.isNotEmpty()) }
    var index by rememberSaveable(semester) { mutableIntStateOf(0) }
    val at = index.coerceIn(0, slides.lastIndex)
    val slide = slides[at]
    val hide = semester in Settings.wrappedHideGrades
    var capture by remember { mutableStateOf<Capture?>(null) }
    LaunchedEffect(slide.name, slides.size) {
        StateRegistry.report("ui.wrapped", mapOf("slide" to slide.name, "index" to at, "slides" to slides.size, "hide" to hide))
    }
    fun go(to: Int) {
        if (to !in slides.indices || to == at) return
        index = to
        haptics.tick()
    }
    val ink = inks(slide.ink)
    val bg by animateColorAsState(ink.bg, MaterialTheme.motionScheme.defaultEffectsSpec())
    val drag = remember { Animatable(0f) }
    val close = remember(context) { { context.activity()?.let { @Suppress("DEPRECATION") it.onBackPressed() }; Unit } }
    Box(
        Modifier.fillMaxSize()
            .graphicsLayer {
                translationY = drag.value
                val s = 1f - (drag.value / size.height).coerceIn(0f, 1f) * 0.12f
                scaleX = s
                scaleY = s
                shape = RoundedCornerShape(if (drag.value > 0f) 28.dp else 0.dp)
                clip = true
            }
            .background(bg)
            .pointerInput(Unit) {
                val limit = 140.dp.toPx()
                detectVerticalDragGestures(
                    onDragEnd = { scope.launch { if (drag.value > limit) close() else drag.animateTo(0f) } },
                    onDragCancel = { scope.launch { drag.animateTo(0f) } },
                ) { change, dy ->
                    change.consume()
                    scope.launch { drag.snapTo((drag.value + dy).coerceAtLeast(0f)) }
                }
            }
            .testTag("wrapped.story"),
    ) {
        // The share image, drawn off screen while Share runs.
        capture?.let { CaptureBox(it) }
        // Tap zones under the slide: back on the left third, on for the rest.
        Row(Modifier.fillMaxSize()) {
            Box(
                Modifier.weight(1f).fillMaxHeight().testTag("wrapped.back")
                    .semantics { contentDescription = "Previous slide" }
                    .clickable(interactionSource = null, indication = null) { go(at - 1) },
            )
            Box(
                Modifier.weight(2f).fillMaxHeight().testTag("wrapped.next")
                    .semantics { contentDescription = "Next slide" }
                    .clickable(interactionSource = null, indication = null) { go(at + 1) },
            )
        }
        val spatial = MaterialTheme.motionScheme.defaultSpatialSpec<Float>()
        val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
        Fit(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.systemBars)) {
            AnimatedContent(
                at,
                transitionSpec = {
                    val forward = targetState > initialState
                    (fadeIn(effects) + scaleIn(spatial, initialScale = if (forward) 0.92f else 1.06f)) togetherWith
                        (fadeOut(effects) + scaleOut(spatial, targetScale = if (forward) 1.06f else 0.92f))
                },
                label = "slide",
            ) { i ->
                SlideCanvas(slides[i], SlideContext(report, name, share = false, hide = false, campus = campus))
            }
        }
        Bars(slides.size, at, ink, Modifier.windowInsetsPadding(WindowInsets.systemBars).padding(start = 16.dp, top = 14.dp, end = 16.dp))
        Row(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth().windowInsetsPadding(WindowInsets.systemBars).padding(start = 24.dp, end = 20.dp, bottom = 22.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (slide.grades) {
                Switch(
                    hide,
                    { on ->
                        Settings.wrappedHideGrades = if (on) Settings.wrappedHideGrades + semester else Settings.wrappedHideGrades - semester
                        haptics.toggle(on)
                        scope.logAction("wrapped.hide_grades", "ui.wrapped", "hide grades ${if (on) "on" else "off"}")
                    },
                    Modifier.testTag("wrapped.hide_grades").semantics { contentDescription = "Hide grades in the shared image" },
                    colors = SwitchDefaults.colors(
                        checkedTrackColor = ink.shape, checkedThumbColor = ink.onShape, checkedBorderColor = ink.shape,
                        uncheckedTrackColor = Color.Transparent, uncheckedThumbColor = ink.text.copy(alpha = 0.7f), uncheckedBorderColor = ink.text.copy(alpha = 0.6f),
                    ),
                )
                Text("Hide grades", Modifier.padding(start = 10.dp), color = ink.text, style = type(14f, 600, -0.01f, 1.2f))
            }
            Spacer(Modifier.weight(1f))
            if (slide.shareable(hide)) Button(
                {
                    if (capture != null) return@Button
                    val shot = Capture(slide, SlideContext(report, name, share = true, hide = hide, campus = campus))
                    capture = shot
                    scope.launch {
                        try {
                            flow("wrapped.share") { f ->
                                val image = withTimeoutOrNull(5_000) { shot.image.await() }
                                if (image == null) {
                                    Log.warn("ui.wrapped", "share image not drawn in time", f, mapOf("slide" to slide.name))
                                    haptics.reject()
                                    showToast("Couldn't share", "The image wasn't ready. Try again.", tone = ToastTone.Error)
                                    return@flow
                                }
                                shareImage(context, image, semester, slide.name)
                                Log.info("ui.wrapped", "share", f, mapOf("slide" to slide.name, "hide" to "$hide", "named" to "${name != null}"))
                            }
                        } catch (e: CancellationException) {
                            throw e
                        } catch (_: Throwable) {
                            // In the log already (the flow's error line).
                            haptics.reject()
                            showToast("Couldn't share", "See flow wrapped.share in the Inspector.", tone = ToastTone.Error)
                        } finally {
                            capture = null
                        }
                    }
                },
                Modifier.height(56.dp).testTag("wrapped.share").semantics { contentDescription = "Share this slide as an image" },
                enabled = capture == null,
                colors = ButtonDefaults.buttonColors(containerColor = ink.shape, contentColor = ink.onShape),
                contentPadding = PaddingValues(start = 20.dp, end = 24.dp),
            ) {
                Icon(Lucide.Share, null, Modifier.size(20.dp))
                Spacer(Modifier.width(10.dp))
                Text("Share", style = type(16f, 700, -0.01f, 1f))
            }
        }
    }
}

/** One segment per slide; those passed filled, the current one filling in. */
@Composable
private fun Bars(count: Int, at: Int, ink: Inks, modifier: Modifier) {
    val fill = remember { Animatable(0f) }
    val spec = MaterialTheme.motionScheme.slowEffectsSpec<Float>()
    LaunchedEffect(at) {
        fill.snapTo(0f)
        fill.animateTo(1f, spec)
    }
    Row(modifier.fillMaxWidth().height(3.dp), horizontalArrangement = Arrangement.spacedBy(3.dp)) {
        repeat(count) { i ->
            Box(Modifier.weight(1f).fillMaxHeight().background(ink.text.copy(alpha = 0.28f), RoundedCornerShape(3.dp))) {
                val part = when {
                    i < at -> 1f
                    i == at -> fill.value
                    else -> 0f
                }
                if (part > 0f) Box(Modifier.fillMaxWidth(part).fillMaxHeight().background(ink.text, RoundedCornerShape(3.dp)))
            }
        }
    }
}

/** [content] in the 360 × 640 design frame, scaled to fit and centred. */
@Composable
private fun Fit(modifier: Modifier, content: @Composable () -> Unit) {
    BoxWithConstraints(modifier, Alignment.Center) {
        val scale = min(constraints.maxWidth / DESIGN_WIDTH, constraints.maxHeight / DESIGN_HEIGHT)
        CompositionLocalProvider(LocalDensity provides Density(scale, 1f), content = content)
    }
}

/**
 * [slide] in its design frame: its tone, the semester line, its content; in a share image also
 * the first name and the vitapmate footer.
 */
@Composable
internal fun SlideCanvas(slide: Slide, context: SlideContext) {
    val ink = inks(slide.ink)
    CompositionLocalProvider(LocalInks provides ink) {
        Box(Modifier.size(DESIGN_WIDTH.dp, DESIGN_HEIGHT.dp).background(ink.bg).clipToBounds().testTag("wrapped.slide.${slide.name}")) {
            if (slide.semesterLine) {
                val line = if (context.share && context.name != null) "${context.name}'s ${context.report.semesterName}" else context.report.semesterName
                Text(line, Modifier.padding(start = 24.dp, top = if (context.share) 24.dp else 28.dp), color = ink.text.copy(alpha = 0.85f), style = type(13f, 600, -0.01f, 1.2f))
            }
            slide.content(this, context)
            if (context.share) Column(Modifier.align(Alignment.BottomStart).padding(start = 24.dp, bottom = 22.dp)) {
                Text("vitapmate", color = ink.text, style = type(17f, 800, -0.03f, 1.1f))
                Text(PLAY_LINK.removePrefix("https://"), Modifier.padding(top = 2.dp), color = ink.text.copy(alpha = 0.75f), style = type(11f, 500, 0f, 1.2f))
            }
        }
    }
}

private tailrec fun Context.activity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.activity()
    else -> null
}
