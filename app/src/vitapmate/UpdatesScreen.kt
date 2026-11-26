package vitapmate

import androidx.compose.animation.AnimatedVisibility
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
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
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
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.ChevronUp
import com.composables.icons.lucide.ChevronsDown
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
import vitapmate.ui.tappable

/** Route id of More → About. */
const val UPDATES = "updates"

private const val GITHUB = "https://github.com/itsAster0/vitap-mate"
private const val INSTAGRAM = "https://www.instagram.com/itsaster0/"

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

/** Notes shown before "Show more" on a long list. */
private const val NOTES_SHOWN = 3

/** Notes of each section an open earlier release shows before "Show all". */
private const val NOTES_PER_SECTION = 2

/**
 * About: who the app is (with GitHub and Instagram right there), what's new in this version
 * (newest first, by section), earlier versions folded, what the app does, and the credits.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatesScreen() {
    val version = rememberAppVersion()
    val context = LocalContext.current
    val code = remember { runCatching { context.packageManager.getPackageInfo(context.packageName, 0).longVersionCode }.getOrDefault(Long.MAX_VALUE) }
    val loaded = rememberGuide("updates.json") { updates(it, code) }
    Page("About", UPDATES) { padding ->
        LazyColumn(contentPadding = padding) {
            item("hero") { Hero(version) }
            val updates = loaded.value
            if (updates == null) item("loading") {
                Box(Modifier.fillMaxWidth().padding(24.dp), Alignment.Center) {
                    loaded.problem?.let { ProblemView(it, loaded.retry.takeIf { _ -> it.retryable }) } ?: LoadingIndicator()
                }
            }
            updates?.releases?.firstOrNull()?.let { release -> item(release.version) { LatestRelease(release) } }
            val earlier = updates?.releases?.drop(1).orEmpty()
            if (earlier.isNotEmpty()) item("earlier") { EarlierReleases(earlier) }
            if (updates != null && updates.features.isNotEmpty()) item("features") { FeatureSections(updates.features) }
            item("credits") { Credits() }
        }
    }
}

/** The app's mark in a cookie, its name, the version (the 7-tap target that turns Developer on) and the two links. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(version: String?) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val uri = LocalUriHandler.current
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge)
            .background(tones.accentTone.subtle).heroCookie().padding(horizontal = 18.dp, vertical = 22.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.size(96.dp).background(scheme.primary, MaterialShapes.Cookie12Sided.toShape()), Alignment.Center) {
            Icon(painterResource(R.drawable.ic_notification), null, Modifier.size(48.dp), tint = scheme.onPrimary)
        }
        Text("vitapmate", Modifier.padding(top = 14.dp), style = MaterialTheme.typography.headlineLargeEmphasized, color = scheme.onSurface)
        Text("Unofficial client for VIT-AP's VTOP.", style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle, textAlign = TextAlign.Center)
        Row(Modifier.padding(top = 14.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            StatusChip(
                version?.let { "Version $it" } ?: "Version",
                tones.accentTone,
                Modifier.clip(MaterialTheme.shapes.small).clickable(onClick = rememberDeveloperTaps()).testTag("updates.version"),
                Lucide.Sparkles,
                solid = true,
            )
            HeroLink("Source code", "updates.hero.github", Lucide.Github) { uri.openUri(GITHUB) }
            HeroLink("Instagram", "updates.hero.instagram", Lucide.Instagram) { uri.openUri(INSTAGRAM) }
        }
    }
}

/** A round logo button on the hero; [label] is read out. */
@Composable
private fun HeroLink(label: String, tag: String, icon: ImageVector, onClick: () -> Unit) =
    Box(Modifier.size(40.dp).clip(CircleShape).background(AppTheme.colors.card).tappable(onClick, label, Role.Button).testTag(tag), Alignment.Center) {
        Icon(icon, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurface)
    }

/** This version: its summary's first sentence as the headline, the rest folded to two lines, then its notes by section. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LatestRelease(release: Release) {
    val scheme = MaterialTheme.colorScheme
    var picked by rememberSaveable { mutableStateOf<String?>(null) }
    var expanded by rememberSaveable { mutableStateOf(false) }
    var allNotes by rememberSaveable { mutableStateOf(false) }
    val section = picked?.takeIf { p -> release.sections.any { it.first == p } } ?: release.sections.first().first
    // The first sentence leads; a summary of one sentence has no rest.
    val cut = release.summary.indexOf(". ")
    val headline = if (cut < 0) release.summary else release.summary.substring(0, cut + 1)
    val rest = if (cut < 0) null else release.summary.substring(cut + 2)
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
        GroupTitle("What's new in ${release.version}")
        Column(
            Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(AppTheme.colors.card)
                .testTag("updates.release.${release.version}").padding(16.dp),
        ) {
            Text(headline, style = MaterialTheme.typography.titleMediumEmphasized, color = scheme.onSurface)
            if (rest != null) {
                var clipped by remember { mutableStateOf(false) }
                Text(
                    rest,
                    Modifier.padding(top = 6.dp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = scheme.onSurfaceVariant,
                    maxLines = if (expanded) Int.MAX_VALUE else 2,
                    overflow = TextOverflow.Ellipsis,
                    onTextLayout = { if (!expanded) clipped = it.hasVisualOverflow },
                )
                if (clipped || expanded) MoreButton(if (expanded) "Show less" else "Read all", expanded, "updates.summary") { expanded = !expanded }
            }
        }
        ConnectedChoice(
            release.sections.map { it.first },
            section,
            { picked = it; allNotes = false },
            tag = "updates.section",
            key = { it },
            label = { title -> "$title · ${release.sections.first { it.first == title }.second.size}" },
            modifier = Modifier.padding(top = 10.dp),
        )
        NoteSegments(release.sections.first { it.first == section }.second, Modifier.padding(top = 10.dp), if (allNotes) null else NOTES_SHOWN) { allNotes = true }
    }
}

/** Older versions, folded; one opens at a time with its summary and a few notes from each section. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun EarlierReleases(releases: List<Release>) {
    var open by rememberSaveable { mutableStateOf<String?>(null) }
    var all by rememberSaveable { mutableStateOf(false) }
    val shapes = listOf(MaterialShapes.Clover4Leaf, MaterialShapes.Cookie7Sided, MaterialShapes.Pill, MaterialShapes.Sunny)
    ListGroup("Earlier versions") {
        releases.forEachIndexed { i, release ->
            ReleaseRow(
                release,
                shapes[i % shapes.size].toShape(),
                open = release.version == open,
                all = all,
                onToggle = { open = if (open == release.version) null else release.version; all = false },
                onShowAll = { all = true },
            )
        }
    }
}

/** One earlier version: a badge with its minor version, its counts, and when [open] its summary and notes. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ReleaseRow(release: Release, shape: Shape, open: Boolean, all: Boolean, onToggle: () -> Unit, onShowAll: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    GroupSegment(Modifier.testTag("updates.release.${release.version}")) {
        Row(
            Modifier.fillMaxWidth().clip(MaterialTheme.shapes.medium).tappable(onToggle, if (open) "${release.version}, open" else release.version, Role.Button),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(48.dp).background(if (open) scheme.primary else scheme.secondaryContainer, shape), Alignment.Center) {
                Text(
                    release.version.substringBeforeLast('.'),
                    style = MaterialTheme.typography.labelLargeEmphasized,
                    color = if (open) scheme.onPrimary else scheme.onSecondaryContainer,
                )
            }
            Column(Modifier.weight(1f)) {
                Text(release.version, style = MaterialTheme.typography.titleMedium, color = scheme.onSurface)
                if (!open) Text(release.summary, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Row(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    release.sections.forEach { (title, notes) -> StatusChip("${notes.size} ${title.lowercase()}", AppTheme.tones.accentTone) }
                }
            }
            Icon(if (open) Lucide.ChevronUp else Lucide.ChevronDown, null, tint = scheme.onSurfaceVariant)
        }
        AnimatedVisibility(open) {
            Column {
                Text(release.summary, Modifier.padding(top = 12.dp), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                release.sections.forEach { (title, notes) ->
                    Row(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(title, style = MaterialTheme.typography.titleSmall, color = scheme.primary)
                        Text("${notes.size}", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                    }
                    (if (all) notes else notes.take(NOTES_PER_SECTION)).forEach { NoteLine(it, Modifier.padding(top = 8.dp)) }
                }
                val total = release.sections.sumOf { it.second.size }
                if (!all && release.sections.any { it.second.size > NOTES_PER_SECTION }) {
                    MoreButton("Show all $total notes", false, "updates.release.${release.version}.all", onShowAll)
                }
            }
        }
    }
}

/** What the app does: a tile per More section (in its shape and first feature's icon), then the picked one's notes. */
@Composable
private fun FeatureSections(features: Sections) {
    var picked by rememberSaveable { mutableStateOf<String?>(null) }
    var all by rememberSaveable { mutableStateOf(false) }
    val feature = picked?.takeIf { p -> features.any { it.first == p } } ?: features.first().first
    Column(Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
        GroupTitle("What's in the app", Modifier.padding(horizontal = 16.dp))
        LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            itemsIndexed(features) { i, (title, notes) ->
                // A section More also has keeps its icon there; the rest get the sparkle.
                val icon = featureGroups.firstOrNull { it.first == title }?.second?.firstOrNull()?.icon ?: Lucide.Sparkles
                SectionTile(title, notes.size, icon, sectionShape(i), title == feature) { picked = title; all = false }
            }
        }
        NoteSegments(
            features.first { it.first == feature }.second,
            Modifier.padding(start = 16.dp, top = 10.dp, end = 16.dp).testTag("updates.features.$feature"),
            if (all) null else NOTES_SHOWN,
        ) { all = true }
    }
}

/** A section of [FeatureSections]: its icon in [shape], its title and how many notes it has; filled when [on]. */
@Composable
private fun SectionTile(title: String, count: Int, icon: ImageVector, shape: Shape, on: Boolean, onClick: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Column(
        Modifier.width(96.dp).clip(RoundedCornerShape(24.dp)).background(if (on) scheme.primaryContainer else AppTheme.colors.card)
            .tappable(onClick, if (on) "$title, shown" else title, Role.Tab).testTag("updates.feature_chip.$title")
            .padding(start = 6.dp, top = 12.dp, end = 6.dp, bottom = 10.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.size(48.dp).background(if (on) scheme.primary else scheme.secondaryContainer, shape), Alignment.Center) {
            Icon(icon, null, Modifier.size(21.dp), tint = if (on) scheme.onPrimary else scheme.onSecondaryContainer)
        }
        Text(
            title,
            Modifier.padding(top = 8.dp),
            style = MaterialTheme.typography.labelLarge,
            color = if (on) scheme.onPrimaryContainer else scheme.onSurface,
            textAlign = TextAlign.Center,
            minLines = 2,
            maxLines = 2,
        )
        Text("$count", style = MaterialTheme.typography.labelMedium, color = if (on) scheme.onPrimaryContainer else scheme.onSurfaceVariant)
    }
}

/** The source code and Instagram tiles (open in the browser), and who builds the app. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Credits() {
    val uri = LocalUriHandler.current
    ListGroup("Credits", footer = "Built by Aster0.") {
        Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
            Tile("Source code", "updates.github", Lucide.Github, CircleShape, "github.com/itsAster0/vitap-mate", false, Modifier.weight(1f), onClick = { uri.openUri(GITHUB) })
            Tile("Instagram", "updates.instagram", Lucide.Instagram, MaterialShapes.Cookie7Sided.toShape(), "@itsaster0", false, Modifier.weight(1f), onClick = { uri.openUri(INSTAGRAM) })
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

/** A primary text button with a chevron that points the way it will fold. */
@Composable
private fun MoreButton(label: String, expanded: Boolean, tag: String, onClick: () -> Unit) = Row(
    Modifier.padding(top = 6.dp).clip(MaterialTheme.shapes.small).tappable(onClick, label, Role.Button).testTag(tag).padding(vertical = 6.dp),
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
) {
    Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
    Icon(if (expanded) Lucide.ChevronUp else Lucide.ChevronDown, null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
}

/** A note with a small Sunny marker in the accent. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NoteLine(note: String, modifier: Modifier = Modifier) = Row(modifier, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
    Box(Modifier.padding(top = 6.dp).size(8.dp).background(AppTheme.tones.accent, MaterialShapes.Sunny.toShape()))
    Text(note, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

/** Notes as one rounded group of segments; past [limit] (null: all) a last segment shows the rest. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NoteSegments(notes: List<String>, modifier: Modifier = Modifier, limit: Int? = null, onShowAll: () -> Unit = {}) {
    val hidden = if (limit == null) 0 else (notes.size - limit).coerceAtLeast(0)
    Column(modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
        notes.take(notes.size - hidden).forEach { GroupSegment { NoteLine(it) } }
        if (hidden > 0) {
            GroupSegment(Modifier.tappable(onShowAll, "Show $hidden more", Role.Button).testTag("updates.notes.more")) {
                Row(Modifier.fillMaxWidth(), Arrangement.Center, Alignment.CenterVertically) {
                    Icon(Lucide.ChevronsDown, null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
                    Text("Show $hidden more", Modifier.padding(start = 6.dp), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                }
            }
        }
    }
}
