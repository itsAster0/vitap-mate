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
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Github
import com.composables.icons.lucide.Instagram
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import org.json.JSONObject
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.ProblemView
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie
import vitapmate.ui.rememberGuide

/** Route id of More → About. */
const val UPDATES = "updates"

internal const val GITHUB = "https://github.com/itsAster0/vitap-mate"
internal const val INSTAGRAM = "https://www.instagram.com/itsaster0/"

/** One version's notes: a line on what it is, then what changed, by section. */
private class Release(val version: String, val summary: String, val sections: Sections)

/** Titled lists of notes. */
private typealias Sections = List<Pair<String, List<String>>>

/**
 * `guides/updates.json` on the edge, for this version: what's new up to it, newest first, and
 * everything it does, in More's sections. The Play Store's short notes are
 * `fastlane/metadata/android/en-GB/changelogs`.
 */
private class Updates(val releases: List<Release>, val features: Sections)

/** [key]'s sections; an item is a string, or `{ text, since }`, left out when [code] is older than `since`. */
private fun JSONObject.sections(key: String, code: Long = Long.MAX_VALUE): Sections {
    val list = getJSONArray(key)
    return List(list.length()) { i ->
        val section = list.getJSONObject(i)
        val items = section.getJSONArray("items")
        section.getString("title") to (0 until items.length()).mapNotNull { j ->
            val item = items.optJSONObject(j) ?: return@mapNotNull items.getString(j)
            item.getString("text").takeIf { item.optLong("since") <= code }
        }
    }.filter { it.second.isNotEmpty() }
}

/** The releases up to this app's [code] (newer ones aren't on this phone) and the features it has. */
private fun updates(json: JSONObject, code: Long): Updates {
    val releases = json.getJSONArray("releases")
    return Updates(
        (0 until releases.length()).map(releases::getJSONObject).filter { it.getLong("code") <= code }
            .map { Release(it.getString("version"), it.getString("summary"), it.sections("sections")) },
        json.sections("features", code),
    )
}

/** About: what's new in each version (newest first, by section), what the app does, and who builds the app with links to GitHub and Instagram (open in the browser). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatesScreen() {
    val uri = LocalUriHandler.current
    val version = rememberAppVersion()
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val context = LocalContext.current
    val code = remember { runCatching { context.packageManager.getPackageInfo(context.packageName, 0).longVersionCode }.getOrDefault(Long.MAX_VALUE) }
    val loaded = rememberGuide("updates.json") { updates(it, code) }
    var picked by rememberSaveable { mutableStateOf<String?>(null) }
    var pickedFeature by rememberSaveable { mutableStateOf<String?>(null) }
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
            val updates = loaded.value
            if (updates == null) item("loading") {
                Box(Modifier.fillMaxWidth().padding(24.dp), Alignment.Center) {
                    loaded.problem?.let { ProblemView(it, loaded.retry.takeIf { _ -> it.retryable }) } ?: LoadingIndicator()
                }
            }
            updates?.releases?.forEachIndexed { i, release ->
                item(release.version) {
                    if (i == 0) {
                        val section = picked?.takeIf { p -> release.sections.any { it.first == p } } ?: release.sections.first().first
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
                                { picked = it },
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
            if (updates != null && updates.features.isNotEmpty()) item("features") {
                val features = updates.features
                val feature = pickedFeature?.takeIf { p -> features.any { it.first == p } } ?: features.first().first
                Column(Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
                    GroupTitle("What's in the app", Modifier.padding(horizontal = 16.dp))
                    LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(features) { (title, _) ->
                            FilterChip(title == feature, { pickedFeature = title }, { Text(title) }, Modifier.testTag("updates.feature_chip.$title"))
                        }
                    }
                    NoteSegments(features.first { it.first == feature }.second, Modifier.padding(start = 16.dp, top = 6.dp, end = 16.dp).testTag("updates.features.$feature"))
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
