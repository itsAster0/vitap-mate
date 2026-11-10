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

/** Everything the app does, in More's sections, then what works around them. */
private val FEATURES = listOf(
    "Classes" to listOf(
        "Timetable shows today's classes with the room, your attendance and how long until each one starts. Swipe through the week or open the weekly grid.",
        "Attendance shows each course's percentage, the classes VTOP hasn't posted yet and a calendar with every class marked.",
        "The attendance calculator tells you how many classes you can miss or must attend to reach your goal, counting OD that's still pending. Plan days lets you pick days to skip or attend and shows where each course ends up.",
        "Course pages list each course's classes, files and course plan. Files download straight into your download folder.",
    ),
    "Exams and results" to listOf(
        "Exams lists the date, time, venue and seat of each exam.",
        "Marks shows every assessment of each course and the total.",
        "Grades shows your grade history and CGPA. The planner works out the grades you need for a target CGPA.",
    ),
    "Campus" to listOf(
        "Calendar shows the academic calendar with holidays, exam days and changed working days.",
        "Outing applies for general and weekend outings, cancels them and opens the pass. A request shows as done only after VTOP confirms it.",
        "Biometric shows when you went in and out on any day.",
    ),
    "Tools" to listOf(
        "Docs lists the files in your download folder and the mess menu, with the mess timings you set.",
        "Profile shows your VTOP profile and your mentor.",
        "VTOP opens the full site, already signed in, for pages the app doesn't cover.",
    ),
    "Around the app" to listOf(
        "Everything stays saved on your phone, so pages open without a connection.",
        "The bell next to the round button with your initial lists what changed on VTOP. Background sync checks every few hours and sends a notification when something changes.",
        "OTP auto-fill reads VTOP's code from your Gmail with your own Google key, or from a forwarding address.",
        "The home-screen widget shows the next class and today's timetable. Long-press the app icon for Timetable, Attendance and Outing.",
        "Live updates put the class that's on now and your outing requests on the lock screen, on Android 16 and later.",
        "In class mode turns on Do Not Disturb while a class is on.",
        "Assistants that support Android's app functions can read your timetable and attendance, on Android 16 and later.",
        "Connected apps pairs the browser extension and makes keys for Claude and other AI apps (MCP).",
        "Share or print the timetable and grade history as a PDF. Long-press the round button with your initial on either page.",
    ),
)

/** What's new, newest first. The Play Store's short notes are `fastlane/metadata/android/en-GB/changelogs`. */
private val RELEASES = listOf(
    Release(
        "3.0.0",
        "vitapmate 3.0 is a new native Android app. It opens faster, scrolls more smoothly and is a smaller download than the old app. " +
            "Everything the old app had is still here, and your data is still saved on your phone.",
        listOf(
            "New" to listOf(
                "The change inbox, background sync and Fetch all, which refreshes every VTOP page at once.",
                "Change notifications have a channel for each kind, and a Mark as seen button.",
                "Attendance calculator with goals, Plan days and the attendance calendar.",
                "Home-screen widget and launcher shortcuts.",
                "Live updates, In class mode and app functions for assistants.",
                "PDFs of the timetable and grade history.",
                "OTP auto-fill from Gmail or a forwarding address. The app stays awake while you sign in to Google, so it finishes before you come back.",
                "Connection, in Settings, reaches VTOP directly, through a compressing proxy or through the vitapmate server. If the server is down, the app goes straight to VTOP.",
                "Connected apps for the browser extension and AI apps. Each key has its own switch for outings, and AI apps can ask for access and you approve it in the app.",
                "The Server page shows what the server keeps and which phones are linked, and lets you clear or delete it.",
                "Your VTOP password is saved in the phone's password manager, and your sign-in survives a reinstall.",
                "A new app icon in five colours, picked in Settings, and a new splash screen.",
                "Updates from Google Play install inside the app.",
            ),
            "Improved" to listOf(
                "Timetable cards put the room and attendance on one line, with a countdown under the times. The timetable also marks Lab FAT days and shows the week ahead.",
                "Attendance shows classes VTOP hasn't posted yet and days without classes.",
                "Downloads go through Android's download manager into the folder you pick, under VTOP's file names. Docs lists what's in that folder.",
                "When a refresh fails, your data stays on screen with the reason and a retry button.",
                "Activity, in Settings, lists what the app fetched from VTOP.",
            ),
        ),
    ),
)

/** About: what's new in each version (newest first, by section), what the app does, and who builds the app with links to GitHub and Instagram (open in the browser). */
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
                        release.sections.forEach { (title, notes) -> Notes(title, notes) }
                    }
                }
            }
            item("features") {
                ListGroup("What's in the app") {
                    FEATURES.forEach { (title, notes) -> Notes(title, notes, Modifier.testTag("updates.features.$title")) }
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

/** A titled list of notes in one segment of a group. */
@Composable
private fun Notes(title: String, notes: List<String>, modifier: Modifier = Modifier) {
    GroupSegment(modifier) {
        Text(title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
        notes.forEach { note ->
            Row(Modifier.padding(top = 8.dp)) {
                Text("•", Modifier.width(16.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(note, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
