package vitapmate

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Github
import com.composables.icons.lucide.Instagram
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles

/** Route id of More → About. */
const val UPDATES = "updates"

private const val GITHUB = "https://github.com/itsKryxen/vitap-mate"
private const val INSTAGRAM = "https://www.instagram.com/itsaster0/"

/** One version's notes: a line on what it is, then what changed, by section. */
private class Release(val version: String, val summary: String, val sections: List<Pair<String, List<String>>>)

/** What's new, newest first. The Play Store's short notes are `fastlane/metadata/android/en-GB/changelogs`. */
private val RELEASES = listOf(
    Release(
        "3.0.0",
        "vitapmate is rebuilt from the ground up as a native Android app. It starts faster, scrolls smoother and is a smaller download. " +
            "Everything you used before is still here and still cached on your phone.",
        listOf(
            "New" to listOf(
                "Change inbox: the bell next to your photo lists what changed on VTOP, such as new marks, attendance, timetable and exam updates.",
                "Background sync checks VTOP every few hours and tells you what changed. With OTP auto-fill on, it can sign in again by itself.",
                "Fetch all, in More, refreshes every VTOP page at once.",
                "Attendance calculator: set a goal and see how many classes you can miss or must attend, with OD that's still pending counted.",
                "Plan days: pick days on a calendar to skip or attend and see where each course ends up.",
                "Attendance calendar with each class marked and a tally for the month.",
                "Connection, in Settings: reach VTOP directly, through a compressing proxy, or let the server fetch pages for you. If the server is down, the app goes straight to VTOP.",
                "Connected apps: pair the browser extension by scanning a QR code, and make keys for Claude and other AI apps (MCP). Each key has its own switch for outings.",
                "Server page: see what the server keeps and which phones are linked, and clear or delete it.",
                "OTP auto-fill, from your Gmail with your own Google key or from a forwarding address.",
            ),
            "Improved" to listOf(
                "Timetable: calmer class cards, Lab FAT days, the week ahead and a ring for the end of the day.",
                "Attendance shows classes VTOP hasn't posted yet and days without classes.",
                "Downloads stay in the folder you pick; Docs shows what's in it.",
                "A failed refresh keeps your data on screen and tells you why, with a retry.",
                "Activity, in Settings, shows what the app fetched from VTOP.",
            ),
        ),
    ),
)

/** About: what's new in each version (newest first, by section), and who builds the app with links to GitHub and Instagram (open in the browser). */
@Composable
fun UpdatesScreen() {
    val uri = LocalUriHandler.current
    val version = rememberAppVersion()
    val open = @Composable { Icon(Lucide.ExternalLink, null, tint = MaterialTheme.colorScheme.onSurfaceVariant) }
    Page("About", UPDATES) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                ListGroup("vitapmate", footer = "Unofficial client for VIT-AP's VTOP.") {
                    GroupItem("Version", "updates.version", Lucide.Sparkles, onClick = rememberDeveloperTaps(), trailing = { version?.let { TrailingValue(it) } })
                }
            }
            RELEASES.forEachIndexed { i, release ->
                item(release.version) {
                    ListGroup(if (i == 0) "What's new in ${release.version}" else release.version) {
                        GroupSegment(Modifier.testTag("updates.release.${release.version}")) {
                            Text(release.summary, style = MaterialTheme.typography.bodyMedium)
                        }
                        release.sections.forEach { (title, notes) ->
                            GroupSegment {
                                Text(title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                                notes.forEach { note ->
                                    Row(Modifier.padding(top = 8.dp)) {
                                        Text("•", Modifier.width(16.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                        Text(note, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            item {
                ListGroup("Credits", footer = "Built by Aster0.") {
                    GroupItem("Source code", "updates.github", Lucide.Github, "github.com/itsKryxen/vitap-mate", onClick = { uri.openUri(GITHUB) }, trailing = open)
                    GroupItem("Instagram", "updates.instagram", Lucide.Instagram, "@itsaster0", onClick = { uri.openUri(INSTAGRAM) }, trailing = open)
                }
            }
        }
    }
}
