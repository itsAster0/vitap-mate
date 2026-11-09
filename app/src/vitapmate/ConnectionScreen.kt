package vitapmate

import androidx.compose.foundation.lazy.LazyColumn
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
import androidx.compose.ui.graphics.vector.ImageVector
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minimize2
import com.composables.icons.lucide.Radio
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.Smartphone
import java.text.DateFormat
import java.util.Date
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.core.Route as VtopRoute
import vitapmate.core.RouteStatus
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.login.ago
import vitapmate.observability.logAction
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

private fun VtopRoute.title() = CHOICES.first { it.route == this }.title

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
            item {
                ListGroup(
                    "How the app reaches VTOP",
                    footer = "If a route through the server fails, the app skips it for 5 minutes and goes straight to VTOP, so you still get your data.",
                ) {
                    CHOICES.forEach { choice ->
                        val selected = status?.chosen == choice.route
                        GroupItem(
                            choice.title,
                            "connection.${choice.route.name.lowercase()}",
                            choice.icon,
                            choice.summary,
                            onClick = { if (!selected) pick(choice.route) },
                            trailing = { RadioButton(selected, { if (!selected) pick(choice.route) }) },
                        )
                    }
                }
            }
            status?.let { current ->
                item {
                    ListGroup("Last refresh") {
                        val used = current.lastUsed
                        val at = current.lastUsedAt
                        GroupItem(
                            if (used != null && at != null) "${used.title()}, ${ago(at)}" else "Nothing fetched since the app opened",
                            "connection.last_used",
                        )
                        current.skipped.forEach { skip ->
                            val until = DateFormat.getTimeInstance(DateFormat.SHORT).format(Date(skip.until * 1000))
                            GroupItem("${skip.route.title()} skipped until $until", "connection.skipped.${skip.route.name.lowercase()}", supporting = skip.reason)
                        }
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
