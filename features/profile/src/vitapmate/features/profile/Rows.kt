package vitapmate.features.profile

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.Eye
import com.composables.icons.lucide.EyeOff
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import com.composables.icons.lucide.MessageCircle
import com.composables.icons.lucide.Phone
import vitapmate.core.Contact
import vitapmate.core.ContactKind
import vitapmate.core.Field
import vitapmate.core.FieldGroup
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.tappable

/** A phone number, email or WhatsApp: tap calls, writes or opens the chat; long press copies. */
@Composable
internal fun ContactRow(contact: Contact, modifier: Modifier = Modifier) {
    val uri = LocalUriHandler.current
    val digits = contact.value.filter(Char::isDigit)
    val (icon, link) = when (contact.kind) {
        ContactKind.EMAIL -> Lucide.Mail to "mailto:${contact.value}"
        // wa.me takes the number with its country code.
        ContactKind.WHATS_APP -> Lucide.MessageCircle to "https://wa.me/${if (digits.length == 10) "91$digits" else digits}"
        ContactKind.PHONE -> Lucide.Phone to "tel:$digits"
    }
    Row(
        modifier.fillMaxWidth().copyable(contact.value) { runCatching { uri.openUri(link) } }.padding(vertical = Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, null, Modifier.size(18.dp), tint = AppTheme.tones.accent)
        Spacer(Modifier.width(Space.md))
        Column(Modifier.weight(1f)) {
            Text(contact.value, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(contact.label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** A label and its value side by side, or stacked when the value is long; Aadhaar masked until tapped. */
@Composable
internal fun FieldRow(field: Field, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    var revealed by rememberSaveable(field.label) { mutableStateOf(!field.secret) }
    val value = if (revealed) field.value else masked(field.raw)
    val label = @Composable { Text(field.label, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant) }
    val shown = @Composable {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(value, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurface)
            if (field.secret) {
                Spacer(Modifier.width(6.dp))
                Icon(if (revealed) Lucide.EyeOff else Lucide.Eye, if (revealed) "Hide" else "Show", Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
            }
        }
    }
    val tap = if (field.secret) ({ revealed = !revealed }) else null
    if (value.length > 22) {
        Column(modifier.fillMaxWidth().copyable(field.raw, tap).padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            label()
            shown()
        }
    } else {
        Row(modifier.fillMaxWidth().copyable(field.raw, tap).padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) { label() }
            Spacer(Modifier.width(Space.md))
            shown()
        }
    }
}

/** Every field by group, the group titles small in the accent. */
@Composable
internal fun Fields(groups: List<FieldGroup>) {
    groups.forEachIndexed { i, group ->
        if (group.title.isNotBlank()) {
            Text(
                group.title.uppercase(),
                Modifier.padding(top = if (i == 0) 0.dp else Space.md, bottom = 2.dp),
                style = MaterialTheme.typography.labelMedium,
                color = AppTheme.tones.accent,
            )
        }
        group.fields.forEach { FieldRow(it) }
    }
}

/** A tappable [label] (with [count] while closed) that shows or hides [content] with a spring. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Expandable(label: String, tag: String, count: Int? = null, header: Boolean = true, content: @Composable () -> Unit) {
    var open by rememberSaveable(tag) { mutableStateOf(false) }
    val motion = MaterialTheme.motionScheme
    val chevron by animateFloatAsState(if (open) 180f else 0f, motion.defaultSpatialSpec())
    val scheme = MaterialTheme.colorScheme
    Column {
        Row(
            Modifier.fillMaxWidth().tappable({ open = !open }, label).padding(vertical = if (header) Space.sm else Space.xs).testTag(tag),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                label,
                Modifier.weight(1f),
                style = if (header) MaterialTheme.typography.titleSmall else MaterialTheme.typography.labelLarge,
                color = if (header) scheme.onSurface else AppTheme.tones.accent,
            )
            if (count != null && !open) Text("$count", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            Spacer(Modifier.width(Space.sm))
            Icon(Lucide.ChevronDown, null, Modifier.size(16.dp).rotate(chevron), tint = scheme.onSurfaceVariant)
        }
        AnimatedVisibility(
            open,
            enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(motion.defaultEffectsSpec()),
            exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut(motion.fastEffectsSpec()),
        ) { Column(Modifier.padding(bottom = Space.sm)) { content() } }
    }
}

/** Everything but the last four characters hidden. */
private fun masked(value: String) = if (value.length <= 4) value else "•".repeat(value.length - 4) + value.takeLast(4)
