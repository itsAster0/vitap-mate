package vitapmate.features.outing

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.DoorOpen
import com.composables.icons.lucide.Lucide
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.OutingFact
import vitapmate.core.OutingKind
import vitapmate.core.OutingRequest
import vitapmate.core.OutingResult
import vitapmate.core.OutingStanding
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.PageHandle
import vitapmate.data.PageState
import vitapmate.data.problemOf
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.LocalOpenDetail
import vitapmate.ui.PageScaffold
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.SectionHeader
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.showToast

/**
 * Writes to VTOP run here, not in a screen's scope: leaving the screen must not cut a request off
 * half sent. Their results only show once VTOP answers (no optimistic rows).
 */
internal val writes = MainScope()

/** What both lists show. */
private class Summary(val open: Boolean, val notice: String, val facts: List<OutingFact>, val requests: List<OutingRequest>)

/** [kind]'s page: its rules panel and requests; New request opens its apply sheet. */
@Composable
internal fun OutingList(kind: OutingKind) {
    var applying by rememberSaveable(kind) { mutableStateOf(false) }
    when (kind) {
        OutingKind.GENERAL -> {
            val page = rememberPage("outing", { c, f -> c.cachedGeneralOuting(f) }, { c, f -> c.fetchGeneralOuting(f) }, { it.updatedAt }, item = kind.key) { it.requests.size }
            Requests(kind, page, { Summary(it.open, it.notice, it.facts, it.requests) }) { applying = true }
            val form = (page.state as? PageState.Ready)?.data
            if (applying && form != null) GeneralSheet(form, onDismiss = { applying = false }, onMaybeSent = { writes.launch { refreshAfterWrite(page, "apply") } }) { result -> sent(page, result); applying = false }
        }
        OutingKind.WEEKEND -> {
            val page = rememberPage("outing", { c, f -> c.cachedWeekendOuting(f) }, { c, f -> c.fetchWeekendOuting(f) }, { it.updatedAt }, item = kind.key) { it.requests.size }
            Requests(kind, page, { Summary(it.open, it.notice, it.facts, it.requests) }) { applying = true }
            val form = (page.state as? PageState.Ready)?.data
            if (applying && form != null) WeekendSheet(form, onDismiss = { applying = false }, onMaybeSent = { writes.launch { refreshAfterWrite(page, "apply") } }) { result -> sent(page, result); applying = false }
        }
    }
}

/** VTOP took the request: say so, then show the list VTOP has now. */
private fun sent(page: PageHandle<*>, result: OutingResult) {
    showToast("Request sent", result.message)
    writes.launch { refreshAfterWrite(page, "apply") }
}

/** Fetches the list again after a write (sent or maybe sent); never throws. */
internal suspend fun refreshAfterWrite(page: PageHandle<*>, trigger: String) {
    try {
        page.refresh(trigger)
    } catch (_: CancellationException) {
        // The screen is gone; its next open fetches.
    }
}

/** What a failed write says: VTOP's or the check's words, else the app's text for the failure. */
internal fun failureOf(e: Throwable, flow: String): String =
    (e as? CoreException.Other)?.reason?.takeIf { it.isNotBlank() } ?: problemOf(e, flow).message

@Composable
private fun <T> Requests(kind: OutingKind, page: PageHandle<T>, summary: (T) -> Summary, onApply: () -> Unit) {
    val scope = rememberCoroutineScope()
    val openDetail = LocalOpenDetail.current
    // Request keys with a cancel or pass on its way; the row stays as it is until VTOP answers.
    var cancelling by remember { mutableStateOf(setOf<String>()) }
    var passLoading by remember { mutableStateOf<String?>(null) }
    var confirm by remember { mutableStateOf<OutingRequest?>(null) }
    val list = rememberLazyListState()
    ScrollToTopOnReselect(list)
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { false },
        empty = {},
    ) { data ->
        val s = summary(data)
        LaunchedEffect(s.requests, s.open, cancelling, passLoading) {
            StateRegistry.report(
                "ui.outing",
                mapOf(
                    "tab" to kind.key,
                    "open" to s.open,
                    "requests" to s.requests.size,
                    "pending" to s.requests.count { it.standing == OutingStanding.PENDING },
                    "cancellable" to s.requests.count { it.cancelId != null },
                    "passes" to s.requests.count { it.passId != null },
                    "cancelling" to cancelling.size,
                    "passLoading" to (passLoading != null),
                ),
            )
        }
        LazyColumn(
            Modifier.fillMaxSize().testTag("outing.list.${kind.key}"),
            list,
            contentPadding = PaddingValues(
                start = Space.sm,
                top = Space.xs,
                end = Space.sm,
                bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
            ),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            item(key = "rules") { RulesPanel(s.open, s.notice, s.facts, onApply, Modifier.animateItem()) }
            item(key = "header") {
                SectionHeader("Your requests", Modifier.animateItem()) { Text("${s.requests.size}") }
            }
            if (s.requests.isEmpty()) item(key = "empty") { EmptyView(Lucide.DoorOpen, "No requests yet", Modifier.animateItem().testTag("outing.empty")) }
            items(s.requests, key = { it.key }) { r ->
                RequestCard(
                    r,
                    onPass = r.passId?.let { passId ->
                        {
                            if (passLoading == null) scope.launch {
                                passLoading = r.key
                                try {
                                    flow("outing.pass") { id ->
                                        Log.info("ui.outing", "pass", id, mapOf("kind" to kind.key))
                                        passDoc(kind, passId, id)
                                    }
                                    openDetail.open("${kind.key}:$passId", "Outing pass")
                                } catch (e: CancellationException) {
                                    throw e
                                } catch (e: Throwable) {
                                    if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason) else showToast("Couldn't get the pass", failureOf(e, ""))
                                } finally {
                                    passLoading = null
                                }
                            }
                        }
                    },
                    passLoading = passLoading == r.key,
                    onCancel = r.cancelId?.let { { confirm = r } },
                    cancelling = r.key in cancelling,
                    modifier = Modifier.animateItem().testTag("outing.request.${r.key}"),
                )
            }
            item(key = "footer") { UpdatedFooter(Modifier.animateItem()) }
        }
    }
    confirm?.let { r ->
        CancelDialog(r, onDismiss = { confirm = null }) {
            confirm = null
            val cancelId = r.cancelId ?: return@CancelDialog
            cancelling = cancelling + r.key
            writes.launch {
                try {
                    val result = flow("outing.cancel") { id ->
                        Log.info("ui.outing", "cancel", id, mapOf("kind" to kind.key))
                        withContext(Dispatchers.IO) { AppCore.await().cancelOuting(id, kind, cancelId) }
                    }
                    showToast(if (result.done) "Request cancelled" else "Couldn't cancel", result.message)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason) else showToast("Couldn't cancel", failureOf(e, ""))
                }
                // Sent or maybe sent: show what VTOP has now, then let the row go.
                refreshAfterWrite(page, "cancel")
                cancelling = cancelling - r.key
            }
        }
    }
}
