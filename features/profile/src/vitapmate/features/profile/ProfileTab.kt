package vitapmate.features.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BedDouble
import com.composables.icons.lucide.House
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import vitapmate.core.Parent
import vitapmate.core.Profile
import vitapmate.ui.AppTheme
import vitapmate.ui.Avatar
import vitapmate.ui.AvatarMenu
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.tappable

/** The profile tab: header, hostel, contacts, addresses, parents, then every other section closed. */
@Composable
internal fun ProfileTab(profile: Profile) {
    LazyColumn(
        Modifier.fillMaxSize().testTag("profile.info"),
        contentPadding = PaddingValues(start = Space.sm, end = Space.sm, top = Space.xs, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl),
    ) {
        item("hero") { Hero(profile) }
        profile.hostel?.let { hostel ->
            item("hostel") {
                Card(Modifier.padding(top = Space.sm).testTag("profile.hostel")) {
                    Headline(AppTheme.tones.lab, hostel.title, listOfNotNull(hostel.line), icon = Lucide.BedDouble)
                }
            }
        }
        if (profile.contacts.isNotEmpty()) {
            item("contacts") {
                SectionTitle("Contact")
                Card(Modifier.testTag("profile.contacts")) { profile.contacts.forEach { ContactRow(it) } }
            }
        }
        if (profile.addresses.isNotEmpty()) {
            item("addresses") {
                SectionTitle("Addresses")
                Card(Modifier.testTag("profile.addresses")) {
                    profile.addresses.forEachIndexed { i, address ->
                        if (i > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Row(Modifier.fillMaxWidth().copyable(address.text).padding(vertical = Space.sm)) {
                            Icon(if (i == 0) Lucide.MapPin else Lucide.House, null, Modifier.padding(top = 2.dp).size(18.dp), tint = AppTheme.tones.accent)
                            Spacer(Modifier.width(Space.md))
                            Column {
                                Text(address.title, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(address.text, style = MaterialTheme.typography.bodyMedium)
                            }
                        }
                    }
                }
            }
        }
        if (profile.parents.isNotEmpty()) {
            item("parents") {
                SectionTitle("Parents")
                Column(verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                    profile.parents.forEachIndexed { i, parent -> ParentCard(parent, i) }
                }
            }
        }
        if (profile.details.isNotEmpty()) {
            item("details") {
                SectionTitle("More details")
                Column(verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                    profile.details.forEachIndexed { i, section ->
                        Card {
                            Expandable(section.title, "profile.details.$i", section.count) { Fields(section.groups) }
                        }
                    }
                }
            }
        }
        item("updated") { UpdatedFooter() }
    }
}

/** The avatar (tap: Choose photo / Use VTOP photo / Remove photo), name, regno, programme and school. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(profile: Profile) {
    var menu by remember { mutableStateOf(false) }
    val scheme = MaterialTheme.colorScheme
    Card(Modifier.testTag("profile.hero")) {
        Row(Modifier.padding(vertical = Space.xs), horizontalArrangement = Arrangement.spacedBy(Space.lg), verticalAlignment = Alignment.CenterVertically) {
            Box {
                Avatar(
                    profile.initials,
                    84.dp,
                    Modifier.tappable({ menu = true }, "Profile photo", Role.Button).testTag("profile.avatar"),
                    shape = MaterialShapes.Cookie9Sided.toShape(),
                    style = MaterialTheme.typography.headlineSmallEmphasized,
                )
                AvatarMenu(menu, { menu = false }, profile.photo)
            }
            Column(Modifier.weight(1f)) {
                Text(profile.name, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface)
                if (profile.registrationNumber.isNotBlank()) {
                    Spacer(Modifier.height(6.dp))
                    StatusChip(profile.registrationNumber, AppTheme.tones.accentTone, Modifier.copyable(profile.registrationNumber))
                }
                if (profile.program.isNotBlank()) {
                    Spacer(Modifier.height(Space.sm))
                    Text(profile.program, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurface)
                }
                if (profile.school.isNotBlank()) Text(profile.school, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            }
        }
    }
}

/** A parent or guardian: name and relation, then mobile and email, the rest behind More. */
@Composable
private fun ParentCard(parent: Parent, index: Int) {
    Card(Modifier.testTag("profile.parent.$index")) {
        Headline(
            AppTheme.tones.warning,
            parent.name ?: parent.relation,
            listOf(listOfNotNull(parent.relation, parent.occupation).joinToString(" · ")),
            initials = parent.initials,
        )
        parent.contacts.forEach { ContactRow(it) }
        if (parent.more.isNotEmpty()) {
            Expandable("More", "profile.parent.$index.more", header = false) { parent.more.forEach { FieldRow(it) } }
        }
    }
}
