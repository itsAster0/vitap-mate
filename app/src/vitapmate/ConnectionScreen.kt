package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.toShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minimize2
import com.composables.icons.lucide.Radio
import com.composables.icons.lucide.School
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.Smartphone
import com.composables.icons.lucide.TriangleAlert
import java.text.DateFormat
import java.util.Date
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.core.Route as VtopRoute
import vitapmate.core.SkippedRoute
import vitapmate.core.RouteStatus
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.login.ago
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.Dot
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie
import vitapmate.ui.neutralTone
import vitapmate.ui.rememberHaptics

/** Route id of Settings → Connection. */
const val CONNECTION = "connection"

private class RouteChoice(val route: VtopRoute, val title: String, val icon: ImageVector, val summary: String)

private val CHOICES = listOf(
    RouteChoice(VtopRoute.DIRECT, "Direct", Lucide.Smartphone, "The phone signs in and fetches from VTOP itself"),
    RouteChoice(VtopRoute.PROXY, "Compressing proxy", Lucide.Minimize2, "The same requests through the vitapmate server, about 6× smaller"),
    RouteChoice(VtopRoute.SERVER, "Server fetch", Lucide.Server, "The server fetches pages; the phone gets only the data"),
    RouteChoice(VtopRoute.UDP, "Server fetch over UDP", Lucide.Radio, "Server fetch over vitapmate's UDP link, for slow or lossy networks"),
)

fun VtopRoute.title() = CHOICES.first { it.route == this }.title

/**
 * How the app reaches VTOP (Rust's `route.rs`): the chosen route, which one the last answer came
 * by and the routes skipped after failing. Any route falls back to the next and finally to Direct,
 * so data still comes when the server is down. Server fetch asks first: the server signs in with
 * the saved password (sent each time, not kept). Flows `connection.*`.
 */
@Composable
fun ConnectionScreen() {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var status by remember { mutableStateOf<RouteStatus?>(null) }
    var asking by remember { mutableStateOf<VtopRoute?>(null) }
    suspend fun load() {
        status = orNull { coreFlow("connection.status") { c, f -> c.routeStatus(f) } }
    }
    LaunchedEffect(Unit) {
        while (true) {
            load()
            delay(5_000)
        }
    }
    fun choose(route: VtopRoute) {
        haptics.toggle(true)
        scope.logAction("connection.choose", "ui.connection", "route ${route.name.lowercase()}")
        scope.launch {
            orNull { coreFlow("connection.set") { c, f -> c.setRoute(f, route) } }
            load()
        }
    }
    val pick = { route: VtopRoute ->
        if (route == VtopRoute.SERVER || route == VtopRoute.UDP) asking = route else choose(route)
    }

    Page("Connection", CONNECTION) { padding ->
        LazyColumn(contentPadding = padding) {
            status?.let { current -> item { RouteHero(current) } }
            item {
                ListGroup(
                    "How the app reaches VTOP",
                    footer = "If a route through the server fails, the app skips it for 5 minutes and goes straight to VTOP, so you still get your data.",
                ) {
                    CHOICES.forEach { choice ->
                        val selected = status?.chosen == choice.route
                        val skip = status?.skipped?.firstOrNull { it.route == choice.route }
                        val signsIn = choice.route == VtopRoute.SERVER || choice.route == VtopRoute.UDP
                        GroupItem(
                            choice.title,
                            "connection.${choice.route.name.lowercase()}",
                            choice.icon,
                            choice.summary,
                            onClick = { if (!selected) pick(choice.route) },
                            trailing = { RadioButton(selected, { if (!selected) pick(choice.route) }) },
                            below = if (skip == null && !signsIn) null else {
                                {
                                    if (signsIn) {
                                        StatusChip("Server signs in", if (selected) AppTheme.tones.accentTone else neutralTone, Modifier.padding(top = 6.dp), solid = selected)
                                    }
                                    skip?.let { SkipNote(it) }
                                }
                            },
                            highlight = selected,
                            iconShape = if (selected) RoundedCornerShape(14.dp) else null,
                        )
                    }
                }
            }
        }
    }

    asking?.let { route ->
        AlertDialog(
            onDismissRequest = { asking = null },
            title = { Text("Let the server sign in?") },
            text = {
                Text(
                    "With ${route.title()}, the vitapmate server signs in to VTOP for you when your session runs out. " +
                        "Your username and password are sent to the server each time it signs in; the server uses them for that sign-in only and doesn't keep them. " +
                        "Changing what VTOP holds (outings) and downloads still happen on your phone.",
                )
            },
            confirmButton = { TextButton({ asking = null; choose(route) }) { Text("Use ${route.title()}") } },
            dismissButton = { TextButton({ asking = null }) { Text("Cancel") } },
        )
    }
}

/** The chosen route as a path (phone, server, VTOP) and the last refresh, in the hero style. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun RouteHero(status: RouteStatus) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val route = status.chosen
    val (first, second) = when (route) {
        VtopRoute.DIRECT -> null to null
        VtopRoute.PROXY -> "6× smaller" to null
        VtopRoute.SERVER -> "data only" to "fetches"
        VtopRoute.UDP -> "UDP" to "fetches"
    }
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge)
            .background(tones.accentTone.subtle).heroCookie().padding(18.dp).testTag("connection.hero"),
    ) {
        Text("Chosen route", style = MaterialTheme.typography.labelLarge, color = tones.accentTone.onSubtle)
        Text(route.title(), style = MaterialTheme.typography.headlineSmallEmphasized, color = scheme.onSurface)
        Row(Modifier.fillMaxWidth().padding(vertical = 16.dp)) {
            PathNode(Lucide.Smartphone, "This phone", CircleShape)
            PathLink(first, dashed = route == VtopRoute.UDP)
            PathNode(Lucide.Server, "vitapmate server", MaterialShapes.Cookie7Sided.toShape(), dim = route == VtopRoute.DIRECT)
            PathLink(second)
            PathNode(Lucide.School, "VTOP", CircleShape)
        }
        val used = status.lastUsed
        val at = status.lastUsedAt
        Row(
            Modifier.fillMaxWidth().background(AppTheme.colors.card, MaterialTheme.shapes.large).padding(horizontal = 12.dp, vertical = 10.dp).testTag("connection.last_used"),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Dot(size = 8.dp, color = if (used != null && at != null) tones.success.base else scheme.outlineVariant)
            Text(
                if (used != null && at != null) "Last refresh: ${used.title()}, ${ago(at)}" else "Nothing fetched since the app opened",
                style = MaterialTheme.typography.bodyMedium,
                color = scheme.onSurface,
            )
        }
    }
}

/** A stop on [RouteHero]'s path; [dim] when the route skips it. */
@Composable
private fun PathNode(icon: ImageVector, label: String, shape: Shape, dim: Boolean = false) {
    val scheme = MaterialTheme.colorScheme
    val alpha = if (dim) 0.38f else 1f
    Column(Modifier.width(64.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(5.dp)) {
        // Opaque even when dim, so the link behind it stays hidden.
        Box(Modifier.size(40.dp).background(AppTheme.colors.card, shape), Alignment.Center) {
            Icon(icon, null, Modifier.size(20.dp), tint = scheme.onSurface.copy(alpha = alpha))
        }
        Text(label, style = MaterialTheme.typography.labelSmall, color = AppTheme.tones.accentTone.onSubtle.copy(alpha = alpha), textAlign = TextAlign.Center)
    }
}

/** The line between two [PathNode]s at their icons' height, with an optional [label] under it. */
@Composable
private fun RowScope.PathLink(label: String?, dashed: Boolean = false) {
    val accent = AppTheme.tones.accent
    Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            Modifier.fillMaxWidth().height(40.dp).drawBehind {
                val y = size.height / 2
                val stroke = 3.dp.toPx()
                drawLine(
                    accent,
                    Offset(-4.dp.toPx(), y),
                    Offset(size.width + 4.dp.toPx(), y),
                    stroke,
                    StrokeCap.Round,
                    if (dashed) PathEffect.dashPathEffect(floatArrayOf(6.dp.toPx(), 5.dp.toPx())) else null,
                )
            },
        )
        label?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = accent, maxLines = 1) }
    }
}

/** A skipped route's warning on its row: until when, and why. */
@Composable
private fun SkipNote(skip: SkippedRoute) {
    val warning = AppTheme.tones.warning
    val until = DateFormat.getTimeInstance(DateFormat.SHORT).format(Date(skip.until * 1000))
    Row(
        Modifier.padding(top = 8.dp).fillMaxWidth().background(warning.subtle, RoundedCornerShape(14.dp)).padding(horizontal = 11.dp, vertical = 9.dp)
            .testTag("connection.skipped.${skip.route.name.lowercase()}"),
        horizontalArrangement = Arrangement.spacedBy(9.dp),
    ) {
        Icon(Lucide.TriangleAlert, null, Modifier.size(16.dp).padding(top = 1.dp), tint = warning.base)
        Column {
            Text("Skipped until $until", style = MaterialTheme.typography.labelLarge, color = warning.onSubtle)
            Text(skip.reason, style = MaterialTheme.typography.bodySmall, color = warning.onSubtle)
        }
    }
}
