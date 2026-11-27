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
import androidx.compose.foundation.border
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.TextButton
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.unit.Dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Download
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
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.WindowInsetsSides
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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.work.WorkInfo
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Share
import com.composables.icons.lucide.Sparkles
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.min
import kotlin.math.roundToInt
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

/** The design frame's width (the mockups' phone); a share image is laid out at it. */
internal const val DESIGN_WIDTH = 360f

/** The screen's scale over the design frame: width / 360, kept in this range (see [Frame]). */
private const val MIN_SCALE = 0.94f
private const val MAX_SCALE = 1.2f

/** The widest frame: a wider screen (a tablet, an unfolded phone) centres it. */
private const val MAX_FRAME_WIDTH = 420f

/** What the story draws from: the report, the class days still without campus times, the first name. */
private class Loaded(val report: WrappedReport?, val missing: List<Int>, val name: String?)

/** How far opening got; the story draws over any of these once there is a report. */
private sealed interface Phase {
    /** The cache being read and what's missing fetched. */
    data object Loading : Phase

    /** Nothing more to fetch: the story, or "Nothing to wrap yet" without a report. */
    data object Done : Phase

    /** Fetching or reading failed; shown only while there is no report. */
    class Failed(val problem: Problem) : Phase
}

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
    var phase by remember(semester, attempt) { mutableStateOf<Phase>(Phase.Loading) }
    LaunchedEffect(semester, attempt) {
        if (semester == null) {
            phase = Phase.Done
            return@LaunchedEffect
        }
        if (semester !in Settings.wrappedSeen) Settings.wrappedSeen += semester
        flow("wrapped.open") { f ->
            var problem: Problem? = null
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
            if (reread()) {
                problem = fetchMissing(f, semester, loaded?.report)
                reread()
                Log.info("ui.wrapped", "open", f, mapOf("report" to "${loaded?.report != null}", "missing_days" to "${loaded?.missing?.size}", "problem" to "${problem?.kind}"))
            }
            phase = problem?.let(Phase::Failed) ?: Phase.Done
        }
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
                    // While opening still runs, its own outcome follows.
                    if (loaded?.report == null && phase != Phase.Loading) phase = Phase.Failed(problemOf(e, f))
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
        phase == Phase.Loading -> Box(Modifier.fillMaxSize().background(Ink.Accent.tone().subtle), Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
                LoadingIndicator(Modifier.size(64.dp))
                Text("Putting your semester together…", style = MaterialTheme.typography.titleMediumEmphasized, color = Ink.Accent.tone().onSubtle)
            }
        }
        else -> Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surface).windowInsetsPadding(WindowInsets.systemBars), Alignment.Center) {
            val p = phase
            if (p is Phase.Failed) ProblemView(p.problem, { attempt++ })
            else EmptyView(
                Lucide.Sparkles,
                "Nothing to wrap yet",
                Modifier.testTag("wrapped.empty"),
                message = "Wrapped opens once this semester's classes wrap up.",
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
 * go back and the rest to go on, swipe down to close; the summary and the outro last. A slide
 * that waits for the student (the guess, the sealed title) shows Skip in place of Share. Share
 * opens the share sheet: Story or Wallpaper, what to hide (remembered), Save to Docs.
 */
@Composable
private fun Story(report: WrappedReport, semester: String, name: String?, campus: CampusFetch) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val slides = remember(report) { slides(report) }
    var index by rememberSaveable(semester) { mutableIntStateOf(0) }
    val state = remember(semester) { StoryState() }
    val at = index.coerceIn(0, slides.lastIndex)
    val slide = slides[at]
    var privacy by remember(semester) { mutableStateOf(Privacy(semester in Settings.wrappedHideGrades, Settings.wrappedHideName, Settings.wrappedHideHostel)) }
    var sharing by remember { mutableStateOf<Slide?>(null) }
    var capture by remember { mutableStateOf<Capture?>(null) }
    LaunchedEffect(slide.name, slides.size) {
        StateRegistry.report("ui.wrapped", mapOf("slide" to slide.name, "index" to at, "slides" to slides.size, "hide_grades" to privacy.grades, "hide_name" to privacy.name, "hide_hostel" to privacy.hostel))
    }
    fun go(to: Int) {
        if (to !in slides.indices || to == at) return
        index = to
        haptics.tick()
    }
    fun keep(p: Privacy) {
        privacy = p
        Settings.wrappedHideGrades = if (p.grades) Settings.wrappedHideGrades + semester else Settings.wrappedHideGrades - semester
        Settings.wrappedHideName = p.name
        Settings.wrappedHideHostel = p.hostel
    }
    /** Draws [target] as a share image (as [privacy] says) and hands it to [use] in flow `wrapped.<action>`; a failure shows a toast. */
    fun export(target: Slide, wallpaper: Boolean, action: String, use: suspend (ImageBitmap, String) -> Unit) {
        if (capture != null) return
        val shot = Capture(target, SlideContext(report, name, share = true, privacy = privacy, campus = campus, state = state, wallpaper = wallpaper))
        capture = shot
        scope.launch {
            try {
                flow("wrapped.$action") { f ->
                    val image = withTimeoutOrNull(5_000) { shot.image.await() }
                    if (image == null) {
                        Log.warn("ui.wrapped", "share image not drawn in time", f, mapOf("slide" to target.name))
                        haptics.reject()
                        showToast("Couldn't make the image", "It wasn't ready. Try again.", tone = ToastTone.Error)
                        return@flow
                    }
                    use(image, f)
                    Log.info(
                        "ui.wrapped", action, f,
                        mapOf("slide" to target.name, "wallpaper" to "$wallpaper", "hide_grades" to "${privacy.grades}", "hide_name" to "${privacy.name}", "hide_hostel" to "${privacy.hostel}", "named" to "${name != null}"),
                    )
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Throwable) {
                // In the log already (the flow's error line).
                haptics.reject()
                showToast("Couldn't make the image", "Something went wrong. Try again.", tone = ToastTone.Error)
            } finally {
                capture = null
            }
        }
    }
    val share = { target: Slide, wallpaper: Boolean ->
        export(target, wallpaper, "share") { image, _ -> shareImage(context, image, semester, target.name) }
    }
    val save = { target: Slide, wallpaper: Boolean ->
        export(target, wallpaper, "save") { image, f ->
            saveImage(image, semester, report.semesterName, target.name, f)
            haptics.confirm()
            showToast("Saved to Docs", "Find it in Docs any time.")
        }
    }
    val summary = slides.first { it.name == "summary" }
    val outro = Outro(
        share = { sharing = summary },
        save = { save(summary, false) },
        replay = {
            state.reset()
            index = 0
            haptics.tick()
        },
        semesters = SemesterChoice.semesters?.items.orEmpty().filter { it.id != semester }.map { it.id to it.name },
        pick = { id ->
            haptics.tick()
            scope.logAction("wrapped.semester", "ui.wrapped", "switch semester")
            SemesterChoice.pick(id)
        },
    )
    val ink = inks(slide.ink)
    val bg by animateColorAsState(ink.bg, MaterialTheme.motionScheme.defaultEffectsSpec())
    val foot by animateColorAsState(inks(slide.foot ?: slide.ink).bg, MaterialTheme.motionScheme.defaultEffectsSpec())
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
        // The share image, drawn off screen while Share or Save runs.
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
        // The frame above, the Share row below it, so the row never covers a slide.
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.systemBars.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))) {
            Fit(Modifier.fillMaxWidth().weight(1f)) {
                AnimatedContent(
                    at,
                    transitionSpec = {
                        val forward = targetState > initialState
                        (fadeIn(effects) + scaleIn(spatial, initialScale = if (forward) 0.92f else 1.06f)) togetherWith
                            (fadeOut(effects) + scaleOut(spatial, targetScale = if (forward) 1.06f else 0.92f))
                    },
                    label = "slide",
                ) { i ->
                    SlideCanvas(slides[i], SlideContext(report, name, share = false, privacy = Privacy(), campus = campus, state = state, outro = outro))
                }
            }
            // In the tone at the frame's bottom edge, down through the navigation bar.
            Box(Modifier.fillMaxWidth().background(foot).windowInsetsPadding(WindowInsets.navigationBars.only(WindowInsetsSides.Bottom))) {
                if (!slide.outro) Footer(slide, state, ink, busy = capture != null) {
                    haptics.open()
                    sharing = slide
                }
            }
        }
        Bars(slides.size, at, ink, Modifier.windowInsetsPadding(WindowInsets.systemBars).padding(start = 16.dp, top = 14.dp, end = 16.dp))
    }
    sharing?.let { target ->
        ShareSheet(
            target,
            { p, wallpaper -> SlideContext(report, name, share = true, privacy = p, campus = campus, state = state, wallpaper = wallpaper) },
            privacy,
            { p ->
                haptics.tick()
                keep(p)
            },
            busy = capture != null,
            onShare = { wallpaper ->
                sharing = null
                share(target, wallpaper)
            },
            onSave = { wallpaper ->
                sharing = null
                save(target, wallpaper)
            },
            onDismiss = { sharing = null },
        )
    }
}

/**
 * Under the frame: Skip (or Tap to unseal) while [slide] waits for the student; else Share, and
 * on the summary the Grid or Receipt switch beside it.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Footer(slide: Slide, state: StoryState, ink: Inks, busy: Boolean, onShare: () -> Unit) {
    val haptics = rememberHaptics()
    Row(
        Modifier.fillMaxWidth().padding(start = 24.dp, top = 6.dp, end = 20.dp, bottom = 16.dp).height(56.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val pending = slide.pending?.takeIf { it.waiting(state) }
        if (pending != null) {
            Spacer(Modifier.weight(1f))
            TextButton(
                {
                    haptics.tick()
                    pending.skip(state)
                },
                Modifier.height(48.dp).testTag("wrapped.skip"),
                colors = ButtonDefaults.textButtonColors(contentColor = ink.text),
            ) { Text(pending.label, style = type(15f, 700, -0.01f, 1f)) }
            return@Row
        }
        if (slide.name == "summary") {
            SingleChoiceSegmentedButtonRow(Modifier.weight(1f).padding(end = 10.dp)) {
                listOf(false to "Grid", true to "Receipt").forEachIndexed { i, (receipt, label) ->
                    SegmentedButton(
                        state.receipt == receipt,
                        {
                            haptics.tick()
                            state.receipt = receipt
                        },
                        SegmentedButtonDefaults.itemShape(i, 2),
                        Modifier.testTag("wrapped.view.${label.lowercase()}"),
                        icon = {},
                    ) { Text(label, style = type(14f, 700, -0.01f, 1f), maxLines = 1) }
                }
            }
        } else Spacer(Modifier.weight(1f))
        if (slide.canShare(Privacy())) Button(
            onShare,
            Modifier.height(56.dp).testTag("wrapped.share").semantics { contentDescription = "Share this slide as an image" },
            enabled = !busy,
            colors = ButtonDefaults.buttonColors(containerColor = ink.shape, contentColor = ink.onShape),
            contentPadding = PaddingValues(start = 20.dp, end = 24.dp),
        ) {
            Icon(Lucide.Share, null, Modifier.size(20.dp))
            Spacer(Modifier.width(10.dp))
            Text("Share", style = type(16f, 700, -0.01f, 1f), maxLines = 1)
        }
    }
}

/**
 * Share as: a live preview of the image, Story (with the vitapmate link) or Wallpaper (just the
 * slide), what the image hides, then Save to Docs or Share. A slide with nothing left to show
 * after what's hidden says so and can't be shared.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun ShareSheet(
    slide: Slide,
    contextOf: (Privacy, Boolean) -> SlideContext,
    privacy: Privacy,
    onPrivacy: (Privacy) -> Unit,
    busy: Boolean,
    onShare: (Boolean) -> Unit,
    onSave: (Boolean) -> Unit,
    onDismiss: () -> Unit,
) {
    var wallpaper by rememberSaveable { mutableStateOf(false) }
    val colors = MaterialTheme.colorScheme
    ModalBottomSheet(onDismiss, Modifier.testTag("wrapped.sheet"), containerColor = colors.surfaceContainerLow) {
        Column(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, bottom = 20.dp)) {
            Text("Share as", style = MaterialTheme.typography.titleLarge)
            Row(Modifier.fillMaxWidth().padding(top = 14.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Preview(slide, contextOf(privacy, wallpaper), wallpaper, 116.dp)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    listOf(false to ("Story" to "With the vitapmate link"), true to ("Wallpaper" to "Just the slide")).forEach { (wall, text) ->
                        val on = wallpaper == wall
                        // A border and a check as well as the colour: dynamic colours can make the
                        // two containers look alike.
                        val shape = RoundedCornerShape(18.dp)
                        Row(
                            Modifier.fillMaxWidth()
                                .background(if (on) colors.secondaryContainer else colors.surfaceContainerHighest, shape)
                                .border(2.dp, if (on) colors.primary else Color.Transparent, shape)
                                .selectable(on, role = Role.RadioButton) { wallpaper = wall }
                                .testTag("wrapped.sheet.${text.first.lowercase()}")
                                .padding(horizontal = 14.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text(text.first, color = if (on) colors.onSecondaryContainer else colors.onSurface, style = MaterialTheme.typography.titleMedium)
                                Text(text.second, color = if (on) colors.onSecondaryContainer else colors.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                            }
                            if (on) Icon(Lucide.Check, null, Modifier.size(18.dp), tint = colors.primary)
                        }
                    }
                }
            }
            Text("Privacy", Modifier.padding(top = 18.dp), color = colors.onSurfaceVariant, style = MaterialTheme.typography.labelLarge)
            FlowRow(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(
                    Triple("Hide grades", privacy.grades, privacy.copy(grades = !privacy.grades)),
                    Triple("Hide name", privacy.name, privacy.copy(name = !privacy.name)),
                    Triple("Hide hostel & outings", privacy.hostel, privacy.copy(hostel = !privacy.hostel)),
                ).forEach { (label, on, next) ->
                    FilterChip(
                        on, { onPrivacy(next) }, { Text(label) },
                        Modifier.testTag("wrapped.sheet.${label.substringAfter(' ').substringBefore(' ').lowercase()}"),
                        leadingIcon = if (on) ({ Icon(Lucide.Check, null, Modifier.size(16.dp)) }) else null,
                    )
                }
            }
            val ok = slide.canShare(privacy)
            if (!ok) Text(
                "Nothing on this slide is left to show with what you're hiding.",
                Modifier.padding(top = 6.dp), color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium,
            )
            Row(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                FilledTonalButton({ onSave(wallpaper) }, Modifier.weight(1f).height(52.dp).testTag("wrapped.sheet.save"), enabled = ok && !busy) {
                    Icon(Lucide.Download, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Save to Docs", maxLines = 1)
                }
                Button({ onShare(wallpaper) }, Modifier.weight(1f).height(52.dp).testTag("wrapped.sheet.share"), enabled = ok && !busy) {
                    Icon(Lucide.Share, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Share", maxLines = 1)
                }
            }
        }
    }
}

/** The share image of [slide], drawn [width] wide. */
@Composable
private fun Preview(slide: Slide, context: SlideContext, wallpaper: Boolean, width: Dp) {
    val base = LocalDensity.current.density
    val shape = RoundedCornerShape(14.dp)
    Box(Modifier.size(width, width * (CONTENT_HEIGHT + FOOTER_HEIGHT) / DESIGN_WIDTH).clip(shape).border(2.dp, MaterialTheme.colorScheme.primary, shape)) {
        CompositionLocalProvider(LocalDensity provides Density(base * width.value / DESIGN_WIDTH, 1f), LocalFrame provides shareFrame(wallpaper)) {
            SlideCanvas(slide, context)
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

/**
 * [content] in the screen's [Frame]: the density scaled by width / 360 within [MIN_SCALE] to
 * [MAX_SCALE] (text keeps its design size, not the system font size), the frame that space at
 * that scale, centred across up to [MAX_FRAME_WIDTH].
 */
@Composable
private fun Fit(modifier: Modifier, content: @Composable () -> Unit) {
    BoxWithConstraints(modifier, Alignment.TopCenter) {
        val scale = (maxWidth.value / DESIGN_WIDTH).coerceIn(MIN_SCALE, MAX_SCALE)
        val frame = Frame(min(maxWidth.value / scale, MAX_FRAME_WIDTH), maxHeight.value / scale)
        CompositionLocalProvider(
            LocalDensity provides Density(LocalDensity.current.density * scale, 1f),
            LocalFrame provides frame,
            content = content,
        )
    }
}

/**
 * [slide] in the [Frame]: its tone, the semester line, its content; in a share image also the
 * first name (unless hidden) and, but for a wallpaper, the vitapmate footer below the frame. A fixed slide is laid out in 360 × 564,
 * scaled down to fit a smaller frame, centred, a little above the middle of a taller one.
 */
@Composable
internal fun SlideCanvas(slide: Slide, context: SlideContext) {
    val ink = inks(slide.ink)
    val frame = LocalFrame.current
    CompositionLocalProvider(LocalInks provides ink) {
        val footer = context.share && !context.wallpaper
        val tall = frame.height + if (footer) FOOTER_HEIGHT else 0f
        // On screen, shapes run on past the sides into a wide screen's margins, as they would off a phone's edges.
        val clip = if (context.share) Modifier.clipToBounds() else Modifier.drawWithContent {
            clipRect(-size.width, 0f, size.width * 2, size.height) { this@drawWithContent.drawContent() }
        }
        Box(Modifier.size(frame.width.dp, tall.dp).background(ink.bg).then(clip).testTag("wrapped.slide.${slide.name}")) {
            val semester = @Composable {
                if (slide.semesterLine) {
                    val line = context.shownName?.takeIf { context.share }?.let { "$it's ${context.report.semesterName}" } ?: context.report.semesterName
                    Text(line, Modifier.padding(start = 24.dp, top = if (context.share) 24.dp else 28.dp), color = ink.text.copy(alpha = 0.85f), style = type(13f, 600, -0.01f, 1.2f))
                }
            }
            if (slide.fixed) {
                Box(Modifier.size(frame.width.dp, frame.height.dp).fixedFrame(frame)) {
                    semester()
                    slide.content(this, context)
                }
            } else {
                Box(Modifier.size(frame.width.dp, frame.height.dp)) {
                    semester()
                    slide.content(this, context)
                }
            }
            if (footer) Column(Modifier.align(Alignment.BottomStart).padding(start = 24.dp, bottom = 22.dp)) {
                Text("vitapmate", color = ink.text, style = type(17f, 800, -0.03f, 1.1f))
                Text(PLAY_LINK.removePrefix("https://"), Modifier.padding(top = 2.dp), color = ink.text.copy(alpha = 0.75f), style = type(11f, 500, 0f, 1.2f))
            }
        }
    }
}

/** Lays the content out in [DESIGN_WIDTH] × [CONTENT_HEIGHT] and draws it scaled to fit [frame], centred across, its spare height 40% above. */
private fun Modifier.fixedFrame(frame: Frame) = layout { measurable, constraints ->
    val s = minOf(1f, frame.width / DESIGN_WIDTH, frame.height / CONTENT_HEIGHT)
    val w = DESIGN_WIDTH.dp.roundToPx()
    val h = CONTENT_HEIGHT.dp.roundToPx()
    val p = measurable.measure(Constraints.fixed(w, h))
    layout(constraints.maxWidth, constraints.maxHeight) {
        val x = ((constraints.maxWidth - w * s) / 2).roundToInt()
        val y = ((constraints.maxHeight - h * s) * 0.4f).roundToInt().coerceAtLeast(0)
        p.placeWithLayer(x, y) {
            scaleX = s
            scaleY = s
            transformOrigin = TransformOrigin(0f, 0f)
        }
    }
}

private tailrec fun Context.activity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.activity()
    else -> null
}
