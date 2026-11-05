package vitapmate.features.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Inbox
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import com.composables.icons.lucide.UserX
import vitapmate.core.Mentor
import vitapmate.core.MentorMessage
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter

/** The mentor tab: new messages first when there are any, then the mentor's card. */
@Composable
internal fun MentorTab(mentor: Mentor?) {
    val messages = mentor?.messages.orEmpty()
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
            item("none") { EmptyView(Lucide.UserX, "No mentor assigned", Modifier.testTag("profile.mentor.empty")) }
        } else {
            item("mentor") {
                if (messages.isNotEmpty()) SectionTitle("Mentor")
                Card(Modifier.testTag("profile.mentor.card")) {
                    Headline(AppTheme.tones.lab, name, mentor.lines, initials = mentor.initials, size = 52.dp)
                    mentor.cabin?.let { cabin ->
                        Row(Modifier.fillMaxWidth().copyable(cabin).padding(top = Space.sm, bottom = Space.xs), verticalAlignment = Alignment.CenterVertically) {
                            Icon(Lucide.MapPin, null, Modifier.size(18.dp), tint = AppTheme.tones.accent)
                            Spacer(Modifier.width(Space.md))
                            Text("Cabin $cabin", style = MaterialTheme.typography.bodyLarge)
                        }
                    }
                    mentor.contacts.forEach { ContactRow(it) }
                    if (mentor.more.isNotEmpty()) Expandable("More", "profile.mentor.more", header = false) { mentor.more.forEach { FieldRow(it) } }
                }
            }
        }
        if (messages.isEmpty() && mentor != null) {
            item("no_messages") {
                Card(Modifier.padding(top = Space.sm)) {
                    Row(Modifier.padding(vertical = Space.xs), verticalAlignment = Alignment.CenterVertically) {
                        Icon(Lucide.Inbox, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                        Spacer(Modifier.width(Space.md))
                        Text("No messages from your mentor", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
        item("updated") { UpdatedFooter() }
    }
}

/** A message's text, its other columns (date, sender) under it; long press copies. */
@Composable
private fun MessageCard(message: MentorMessage, modifier: Modifier = Modifier) = Card(modifier.copyable(message.body)) {
    Text(message.body, style = MaterialTheme.typography.bodyMedium)
    if (message.meta.isNotEmpty()) {
        Column(Modifier.padding(top = Space.xs)) {
            message.meta.forEach { Text(it, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
    }
}
