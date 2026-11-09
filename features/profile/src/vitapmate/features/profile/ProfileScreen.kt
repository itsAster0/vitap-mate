package vitapmate.features.profile

import com.composables.icons.lucide.Users
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.CircleUser
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.PageScaffold
import vitapmate.ui.Space
import vitapmate.ui.rememberSyncedPager

private enum class Tab { Profile, Mentor }

/**
 * The profile (`page.profile`, log src `ui.profile`): the cached copy first, then VTOP; not per
 * semester. Profile and Mentor over a pager synced with a connected choice. Only counts are
 * logged, never a name, number or address.
 */
@Composable
fun ProfileScreen() {
    val page = rememberPage(
        "profile",
        { core, flow -> core.cachedProfile(flow) },
        { core, flow -> core.fetchProfile(flow) },
        { it.updatedAt },
    ) { it.details.size }
    val scope = rememberCoroutineScope()
    var tab by rememberSaveable { mutableStateOf(Tab.Profile) }
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.name.isBlank() && it.registrationNumber.isBlank() },
        empty = { EmptyView(Lucide.CircleUser, "No profile yet", Modifier.testTag("profile.empty")) },
    ) { profile ->
        val pager = rememberSyncedPager(Tab.entries, tab) { landed ->
            tab = landed
            flow("profile.tab") { id -> Log.info("ui.profile", "tab ${landed.name.lowercase()} (swipe)", id, mapOf("tab" to landed.name.lowercase(), "by" to "swipe")) }
        }
        LaunchedEffect(profile, tab) {
            StateRegistry.report(
                "ui.profile",
                mapOf(
                    "tab" to tab.name.lowercase(),
                    "sections" to profile.details.size,
                    "contacts" to profile.contacts.size,
                    "parents" to profile.parents.size,
                    "hostel" to (profile.hostel != null),
                    "mentor" to (profile.mentor?.name != null),
                    "messages" to (profile.mentor?.messages?.size ?: 0),
                ),
            )
        }
        Column(Modifier.fillMaxSize()) {
            ConnectedChoice(
                Tab.entries,
                Tab.entries[pager.targetPage],
                onSelect = { t ->
                    tab = t
                    scope.launch { pager.animateScrollToPage(t.ordinal) }
                    scope.logAction("profile.tab", "ui.profile", "tab ${t.name.lowercase()}", mapOf("tab" to t.name.lowercase(), "by" to "tap"))
                },
                tag = "profile.tab",
                key = { it.name.lowercase() },
                label = { it.name },
                icon = { if (it == Tab.Profile) Lucide.CircleUser else Lucide.Users },
                labelOnSelected = true,
                modifier = Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.sm),
            )
            HorizontalPager(pager, Modifier.weight(1f).testTag("profile.pager"), key = { Tab.entries[it].name }) { i ->
                when (Tab.entries[i]) {
                    Tab.Profile -> ProfileTab(profile)
                    Tab.Mentor -> MentorTab(profile.mentor)
                }
            }
        }
    }
}
