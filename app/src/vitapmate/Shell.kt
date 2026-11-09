package vitapmate

import androidx.compose.animation.core.FastOutLinearInEasing
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.AnimatedContentTransitionScope
import vitapmate.ui.LocalTabReselect
import kotlinx.coroutines.flow.MutableSharedFlow
import androidx.compose.runtime.remember
import vitapmate.ui.DetailOpener
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.LocalOpenDetail
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.runtime.CompositionLocalProvider
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.ui.unit.IntOffset
import androidx.compose.material3.MaterialTheme
import androidx.compose.animation.scaleOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideInHorizontally
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FloatingToolbarDefaults
import androidx.compose.material3.FloatingToolbarExitDirection
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.layout.findRootCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import vitapmate.ui.NavBarSpace
import kotlin.math.roundToInt
import androidx.compose.runtime.saveable.SaveableStateHolder
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.unit.dp
import androidx.navigation3.runtime.NavEntry
import androidx.navigation3.ui.NavDisplay
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry

/** Signed in: the screen on top of the back stack, the floating nav bar over it on tab roots. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Shell(nav: Nav, onSignedOut: () -> Unit) {
    val tabs = navTabs
    val scroll = FloatingToolbarDefaults.exitAlwaysScrollBehavior(FloatingToolbarExitDirection.Bottom)
    // Keeps each tab's saveable state (scroll, selections) while another tab is shown.
    val tabStates = rememberSaveableStateHolder()
    val reselect = remember { MutableSharedFlow<Unit>(extraBufferCapacity = 1) }
    val keys = nav.stack.map { it.key }
    LaunchedEffect(keys, tabs) {
        scroll.state.offset = 0f
        Log.info("ui.shell", "route ${nav.top.key}", fields = mapOf("tab" to nav.tab.orEmpty(), "stack" to keys.joinToString(">")))
        StateRegistry.report("route", mapOf("route" to nav.top.key, "tab" to nav.tab, "stack" to keys, "tabs" to tabs))
    }
    // The bar's top above the window's bottom, before the scroll offset; NavBarSpace adds it.
    val barTop = remember { mutableIntStateOf(0) }
    LaunchedEffect(scroll) { snapshotFlow { (barTop.intValue + scroll.state.offset).roundToInt().coerceAtLeast(0) }.collect { NavBarSpace.px = it } }
    DisposableEffect(Unit) { onDispose { NavBarSpace.px = 0 } }
    // A change notification was tapped.
    LaunchedEffect(InboxRequest.pending) {
        if (!InboxRequest.pending) return@LaunchedEffect
        InboxRequest.pending = false
        if (nav.top.key != INBOX) nav.push(Route.Screen(INBOX))
    }
    // Back on a tab other than the first goes to the first, then leaves the app.
    BackHandler(nav.stack.size == 1 && nav.root.key != tabs.first()) { nav.goTab(tabs.first()) }
    Box(Modifier.fillMaxSize().nestedScroll(scroll)) {
        // Expressive springs: pushes slide in with a little overshoot; predictive back shrinks the
        // page under your thumb and settles the one below.
        val spatial = MaterialTheme.motionScheme.defaultSpatialSpec<IntOffset>()
        val scale = MaterialTheme.motionScheme.defaultSpatialSpec<Float>()
        val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
        // Going back is quicker than going forward: the page below is already known.
        val backSpatial = MaterialTheme.motionScheme.fastSpatialSpec<IntOffset>()
        val backEffects = MaterialTheme.motionScheme.fastEffectsSpec<Float>()
        NavDisplay(
            backStack = nav.stack,
            onBack = nav::pop,
            transitionSpec = {
                (slideInHorizontally(spatial) { it / 3 } + fadeIn(effects)) togetherWith
                    (slideOutHorizontally(spatial) { -it / 8 } + fadeOut(effects))
            },
            popTransitionSpec = {
                (slideInHorizontally(backSpatial) { -it / 8 } + fadeIn(backEffects)) togetherWith
                    (slideOutHorizontally(backSpatial) { it / 3 } + fadeOut(backEffects))
            },
            predictivePopTransitionSpec = {
                (scaleIn(scale, initialScale = 0.94f) + fadeIn(effects)) togetherWith
                    (scaleOut(scale, targetScale = 0.86f) + fadeOut(effects))
            },
            entryProvider = { route ->
                // Switching tabs fades through; screens pushed on a tab slide in (the default).
                val meta = if (route == nav.root) tabFade else emptyMap()
                NavEntry(route, route.key, meta) {
                    CompositionLocalProvider(LocalTabReselect provides reselect, LocalRoute provides it) { Entry(it, tabStates, onSignedOut) }
                }
            },
        )
        AnimatedVisibility(
            nav.stack.size == 1,
            Modifier.align(Alignment.BottomCenter),
            enter = slideInVertically { it } + fadeIn(),
            exit = slideOutVertically { it } + fadeOut(),
        ) {
            DisposableEffect(Unit) { onDispose { barTop.intValue = 0 } }
            NavBar(
                tabs.mapNotNull(::featureById),
                nav.tab,
                scroll,
                onSelect = { id ->
                    if (id == nav.tab) reselect.tryEmit(Unit) else nav.goTab(id)
                },
                onEdit = { nav.push(Route.Tabs) },
                modifier = Modifier.navigationBarsPadding().padding(bottom = 16.dp)
                    .onGloballyPositioned { barTop.intValue = (it.findRootCoordinates().size.height - it.positionInRoot().y).roundToInt() },
            )
        }
    }
}

// M3 fade through: the old tab fades out fast, the new one fades in and grows from 96% after it.
private val fadeThrough: AnimatedContentTransitionScope<*>.() -> ContentTransform = {
    (fadeIn(tween(210, delayMillis = 90, easing = LinearOutSlowInEasing)) + scaleIn(tween(210, delayMillis = 90, easing = LinearOutSlowInEasing), initialScale = 0.96f)) togetherWith
        fadeOut(tween(90, easing = FastOutLinearInEasing))
}
private val tabFade = NavDisplay.transitionSpec { fadeThrough() } + NavDisplay.popTransitionSpec { fadeThrough() }

@Composable
private fun Entry(route: Route, tabStates: SaveableStateHolder, onSignedOut: () -> Unit) {
    when (route) {
        Route.Tabs -> TabsScreen()
        is Route.FlowLog -> FlowScreen(route.flow)
        is Route.Detail -> {
            val detail = featureById(route.feature)?.detail ?: return Page(route.title, "detail") { ComingSoon(route.feature) }
            Page(route.title, "detail.${route.feature}") { padding ->
                Box(Modifier.padding(top = padding.calculateTopPadding())) {
                    CompositionLocalProvider(LocalContentPadding provides PaddingValues(bottom = padding.calculateBottomPadding())) {
                        detail(route.item)
                    }
                }
            }
        }
        is Route.Screen -> when (route.id) {
            MORE -> MoreScreen()
            SETTINGS -> SettingsScreen(onSignedOut)
            INSPECTOR -> InspectorScreen()
            ACTIVITY -> ActivityScreen()
            ACTIVITY_HISTORY -> ActivityScreen(showOlder = true)
            SYNC_HISTORY -> ActivityScreen(showOlder = true, title = "Sync history", only = { it.title == "Background sync" })
            DEVELOPER -> DeveloperScreen()
            ACCOUNT -> AccountScreen()
            CONNECTED_APPS -> ConnectedAppsScreen()
            SERVER -> ServerScreen()
            CONNECTION -> ConnectionScreen()
            UPDATES -> UpdatesScreen()
            INBOX -> InboxScreen()
            else -> {
                val f = featureById(route.id) ?: return Page(route.id, route.id) { ComingSoon(route.id) }
                if (f.fullScreen) return Box(Modifier.fillMaxSize().testTag("screen.${f.id}")) { f.content() }
                Page(f.label, f.id, f.actions) { padding ->
                    Box(Modifier.padding(top = padding.calculateTopPadding()).padding(horizontal = 8.dp)) {
                        CompositionLocalProvider(LocalContentPadding provides PaddingValues(bottom = padding.calculateBottomPadding())) {
                            val nav = LocalNav.current
                            CompositionLocalProvider(LocalOpenDetail provides DetailOpener { item, title -> nav.push(Route.Detail(f.id, item, title)) }) {
                                tabStates.SaveableStateProvider("tab.${f.id}") { f.content() }
                            }
                        }
                    }
                }
            }
        }
    }
}
