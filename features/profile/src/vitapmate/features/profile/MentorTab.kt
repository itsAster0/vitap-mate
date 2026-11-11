package vitapmate.features.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Inbox
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import com.composables.icons.lucide.UserX
import vitapmate.core.Mentor
import vitapmate.core.MentorMessage
import vitapmate.ui.AppTheme
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space
import vitapmate.ui.copyable
import vitapmate.ui.Tone
import vitapmate.ui.UpdatedFooter

/** The mentor tab: new messages first when there are any, then the mentor's card. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun MentorTab(mentor: Mentor?) {
    val messages = mentor?.messages.orEmpty()
    val scheme = MaterialTheme.colorScheme
    val lab = AppTheme.tones.lab
    // A neutral tone for the empty states' icons.
    val quiet = Tone(scheme.onSurfaceVariant, scheme.surfaceContainerHigh, scheme.onSurface)
    LazyColumn(
        Modifier.fillMaxSize().testTag("profile.mentor"),
        contentPadding = PaddingValues(start = Space.sm, end = Space.sm, top = Space.xs, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        if (messages.isNotEmpty()) {
            item("messages") { SectionTitle("Messages · ${messages.size}") }
            itemsIndexed(messages) { i, message -> MessageCard(message, Modifier.testTag("profile.message.$i")) }
        }
        val name = mentor?.name
        if (mentor == null || name == null) {
            item("none") {
                Column(
                    Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(AppTheme.colors.card).padding(horizontal = Space.lg, vertical = 26.dp)
                        .testTag("profile.mentor.empty"),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    IconCircle(Lucide.UserX, quiet, 56.dp, MaterialShapes.Cookie7Sided.toShape())
                    Text("No mentor assigned", style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant, textAlign = TextAlign.Center)
                }
            }
        } else {
            item("mentor") {
                if (messages.isNotEmpty()) SectionTitle("Mentor")
                Segmented(Modifier.testTag("profile.mentor.card")) {
                    Segment(color = lab.subtle) {
                        Row(Modifier.padding(vertical = Space.xs), horizontalArrangement = Arrangement.spacedBy(Space.lg), verticalAlignment = Alignment.CenterVertically) {
                            Box(Modifier.size(76.dp).background(lab.base, MaterialShapes.Sunny.toShape()), Alignment.Center) {
                                Text(mentor.initials, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.surface)
                            }
                            Column(Modifier.weight(1f)) {
                                Text(name, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface)
                                mentor.lines.forEach { Text(it, Modifier.padding(top = 2.dp), style = MaterialTheme.typography.bodySmall, color = lab.onSubtle) }
                            }
                        }
                        mentor.cabin?.let { cabin ->
                            Row(
                                Modifier.padding(top = Space.md, bottom = Space.xs).clip(CircleShape).background(lab.base).copyable(cabin).height(34.dp).padding(horizontal = 13.dp),
                                horizontalArrangement = Arrangement.spacedBy(7.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Icon(Lucide.MapPin, null, Modifier.size(15.dp), tint = scheme.surface)
                                Text("Cabin $cabin", style = MaterialTheme.typography.labelLargeEmphasized, color = scheme.surface)
                            }
                        }
                    }
                    mentor.contacts.forEach { ContactRow(it, tone = lab) }
                    if (mentor.more.isNotEmpty()) Expandable("More", "profile.mentor.more", header = false, accent = lab.base) { mentor.more.forEach { FieldRow(it) } }
                }
            }
        }
        if (messages.isEmpty() && mentor != null) {
            item("no_messages") {
                Segmented(Modifier.padding(top = Space.sm)) {
                    InfoRow(Lucide.Inbox, quiet, null) {
                        Text("No messages from your mentor", style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                    }
                }
            }
        }
        item("updated") { UpdatedFooter() }
    }
}

/** A message as a received bubble: its text, its other columns (date, sender) under it; long press copies. */
@Composable
private fun MessageCard(message: MentorMessage, modifier: Modifier = Modifier) = Column(
    modifier.padding(end = 28.dp).fillMaxWidth().clip(RoundedCornerShape(26.dp, 26.dp, 26.dp, 8.dp)).background(AppTheme.colors.card)
        .copyable(message.body).padding(horizontal = Space.lg, vertical = 14.dp),
) {
    Text(message.body, style = MaterialTheme.typography.bodyMedium)
    if (message.meta.isNotEmpty()) {
        Column(Modifier.padding(top = Space.sm)) {
            message.meta.forEach { Text(it, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
    }
}
