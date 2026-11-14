package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Github
import com.composables.icons.lucide.Instagram
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie

/** Route id of More → About. */
const val UPDATES = "updates"

private const val GITHUB = "https://github.com/itsAster0/vitap-mate"
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
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatesScreen() {
    val uri = LocalUriHandler.current
    val version = rememberAppVersion()
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    var section by rememberSaveable { mutableStateOf(RELEASES.first().sections.first().first) }
    var feature by rememberSaveable { mutableStateOf(FEATURES.first().first) }
    Page("About", UPDATES) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge)
                        .background(tones.accentTone.subtle).heroCookie().padding(18.dp),
                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(Modifier.size(64.dp).background(scheme.primary, RoundedCornerShape(22.dp)), Alignment.Center) {
                        Icon(painterResource(R.drawable.ic_notification), null, Modifier.size(36.dp), tint = scheme.onPrimary)
                    }
                    Column(Modifier.weight(1f)) {
                        Text("vitapmate", style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface)
                        Text("Unofficial client for VIT-AP's VTOP.", style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle)
                        // The version is the 7-tap target that turns Developer on.
                        StatusChip(
                            version?.let { "Version $it" } ?: "Version",
                            tones.accentTone,
                            Modifier.padding(top = 8.dp).clip(MaterialTheme.shapes.small).clickable(onClick = rememberDeveloperTaps()).testTag("updates.version"),
                            Lucide.Sparkles,
                            solid = true,
                        )
                    }
                }
            }
            RELEASES.forEachIndexed { i, release ->
                item(release.version) {
                    if (i == 0) {
                        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
                            GroupTitle("What's new in ${release.version}")
                            Text(
                                release.summary,
                                Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(tones.accentTone.subtle).testTag("updates.release.${release.version}").padding(16.dp),
                                style = MaterialTheme.typography.bodyMedium,
                                color = tones.accentTone.onSubtle,
                            )
                            ConnectedChoice(
                                release.sections.map { it.first },
                                section,
                                { section = it },
                                tag = "updates.section",
                                key = { it },
                                label = { title -> "$title · ${release.sections.first { it.first == title }.second.size}" },
                                modifier = Modifier.padding(top = 10.dp),
                            )
                            NoteSegments(release.sections.first { it.first == section }.second, Modifier.padding(top = 10.dp))
                        }
                    } else {
                        ListGroup(release.version) {
                            GroupSegment(Modifier.testTag("updates.release.${release.version}")) {
                                Text(release.summary, style = MaterialTheme.typography.bodyMedium)
                            }
                            release.sections.forEach { (title, notes) -> Notes(title, notes) }
                        }
                    }
                }
            }
            item("features") {
                Column(Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
                    GroupTitle("What's in the app", Modifier.padding(horizontal = 16.dp))
                    LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(FEATURES) { (title, _) ->
                            FilterChip(title == feature, { feature = title }, { Text(title) }, Modifier.testTag("updates.feature_chip.$title"))
                        }
                    }
                    NoteSegments(FEATURES.first { it.first == feature }.second, Modifier.padding(start = 16.dp, top = 6.dp, end = 16.dp).testTag("updates.features.$feature"))
                }
            }
            item {
                ListGroup("Credits", footer = "Built by Aster0.") {
                    Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        Tile("Source code", "updates.github", Lucide.Github, CircleShape, "github.com/itsAster0/vitap-mate", false, Modifier.weight(1f), onClick = { uri.openUri(GITHUB) })
                        Tile("Instagram", "updates.instagram", Lucide.Instagram, MaterialShapes.Cookie7Sided.toShape(), "@itsaster0", false, Modifier.weight(1f), onClick = { uri.openUri(INSTAGRAM) })
                    }
                }
            }
        }
    }
}

/** A section title in [ListGroup]'s style, for sections laid out by hand. */
@Composable
private fun GroupTitle(text: String, modifier: Modifier = Modifier) = Text(
    text,
    modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp).semantics { heading() },
    style = MaterialTheme.typography.titleSmall,
    color = MaterialTheme.colorScheme.primary,
)

/** Notes as one rounded group of segments, each with a small accent marker. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NoteSegments(notes: List<String>, modifier: Modifier = Modifier) {
    Column(modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
        notes.forEach { note ->
            GroupSegment {
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Box(Modifier.padding(top = 6.dp).size(8.dp).background(AppTheme.tones.accent, RoundedCornerShape(3.dp)))
                    Text(note, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
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
