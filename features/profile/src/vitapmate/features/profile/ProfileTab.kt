package vitapmate.features.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
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
import androidx.compose.ui.draw.clip
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
import vitapmate.ui.CameraBadge
import vitapmate.ui.heroCookie
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.RegnoPill
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.tappable

/** The profile tab: header and hostel, contacts, addresses, parents, then every other section closed. */
@Composable
internal fun ProfileTab(profile: Profile) {
    LazyColumn(
        Modifier.fillMaxSize().testTag("profile.info"),
        contentPadding = PaddingValues(start = Space.sm, end = Space.sm, top = Space.xs, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl),
    ) {
        item("hero") {
            Segmented {
                Hero(profile)
                profile.hostel?.let { hostel ->
                    Segment(Modifier.testTag("profile.hostel")) {
                        Headline(AppTheme.tones.lab, hostel.title, listOfNotNull(hostel.line), icon = Lucide.BedDouble)
                    }
                }
            }
        }
        if (profile.contacts.isNotEmpty()) {
            item("contacts") {
                SectionTitle("Contact")
                Segmented(Modifier.testTag("profile.contacts")) { profile.contacts.forEach { ContactRow(it) } }
            }
        }
        if (profile.addresses.isNotEmpty()) {
            item("addresses") {
                SectionTitle("Addresses")
                Segmented(Modifier.testTag("profile.addresses")) {
                    profile.addresses.forEachIndexed { i, address ->
                        InfoRow(if (i == 0) Lucide.MapPin else Lucide.House, AppTheme.tones.accentTone, address.text) {
                            Text(address.title, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(address.text, style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }
            }
        }
        if (profile.parents.isNotEmpty()) {
            item("parents") {
                SectionTitle("Parents")
                Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
                    profile.parents.forEachIndexed { i, parent -> ParentCard(parent, i) }
                }
            }
        }
        if (profile.details.isNotEmpty()) {
            item("details") {
                SectionTitle("More details")
                Segmented {
                    profile.details.forEachIndexed { i, section ->
                        Expandable(section.title, "profile.details.$i", section.count) { Fields(section.groups) }
                    }
                }
            }
        }
        item("updated") { UpdatedFooter() }
    }
}

/**
 * The avatar (tap: Choose photo / Use VTOP photo / Remove photo) with a camera badge, the regno
 * as a pill (tap or long press copies), then name, programme and school, on the accent's fill.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(profile: Profile) {
    var menu by remember { mutableStateOf(false) }
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Box(Modifier.fillMaxWidth().clip(SegmentShape).background(tones.accentTone.subtle).heroCookie().testTag("profile.hero")) {
        Column(Modifier.padding(start = 20.dp, top = 20.dp, end = 20.dp, bottom = 22.dp)) {
            Row(verticalAlignment = Alignment.Top) {
                Box {
                    Avatar(
                        profile.initials,
                        104.dp,
                        Modifier.tappable({ menu = true }, "Profile photo", Role.Button).testTag("profile.avatar"),
                        shape = MaterialShapes.Cookie9Sided.toShape(),
                        style = MaterialTheme.typography.displaySmallEmphasized,
                    )
                    CameraBadge(32.dp, Modifier.align(Alignment.BottomEnd).offset(x = 2.dp))
                    AvatarMenu(menu, { menu = false }, profile.photo)
                }
                Spacer(Modifier.weight(1f))
                if (profile.registrationNumber.isNotBlank()) RegnoPill(profile.registrationNumber)
            }
            Text(profile.name, Modifier.padding(top = 18.dp, bottom = Space.md), style = MaterialTheme.typography.headlineLargeEmphasized, color = scheme.onSurface)
            if (profile.program.isNotBlank()) Text(profile.program, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurface)
            if (profile.school.isNotBlank()) Text(profile.school, Modifier.padding(top = 2.dp), style = MaterialTheme.typography.bodySmall, color = tones.accentTone.onSubtle)
        }
    }
}

/** A parent or guardian: name and relation, then mobile and email, the rest behind More. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ParentCard(parent: Parent, index: Int) {
    Segmented(Modifier.testTag("profile.parent.$index")) {
        Segment {
            Headline(
                AppTheme.tones.warning,
                parent.name ?: parent.relation,
                listOf(listOfNotNull(parent.relation, parent.occupation).joinToString(" · ")),
                initials = parent.initials,
                size = 48.dp,
                shape = MaterialShapes.Clover4Leaf.toShape(),
            )
        }
        parent.contacts.forEach { ContactRow(it) }
        if (parent.more.isNotEmpty()) {
            Expandable("More", "profile.parent.$index.more", header = false) { parent.more.forEach { FieldRow(it) } }
        }
    }
}
