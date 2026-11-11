package vitapmate

import vitapmate.data.AccountState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CloudOff
import com.composables.icons.lucide.Lucide
import vitapmate.data.Activity
import vitapmate.data.OtpPrompt
import vitapmate.data.SemesterChoice
import vitapmate.ui.AppTheme
import vitapmate.ui.Avatar
import vitapmate.ui.AvatarMenu
import vitapmate.ui.CameraBadge
import vitapmate.ui.heroCookie
import vitapmate.ui.StatusChip
import vitapmate.ui.shortSemesterName
import vitapmate.ui.tappable

/**
 * The top of Settings: the avatar (the chosen photo or the initial, in an expressive shape; tap
 * for [AvatarMenu]), the name from the cached profile ([name]) over the registration number, the
 * semester pages show, and the account's state (signed in, OTP waiting, offline). [regno] (from
 * the cached profile) is a pill after the username, unless blank or the username itself.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ProfileHeader(user: String?, name: String?, vtopPhoto: String?, regno: String?, modifier: Modifier = Modifier) {
    var menu by remember { mutableStateOf(false) }
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val semester = SemesterChoice.semesters?.let { list -> list.items.firstOrNull { it.id == list.selected } }
    val number = regno?.takeIf { it.isNotBlank() && !it.equals(user, ignoreCase = true) }
    val (state, tone) = when {
        user == null -> "Not signed in" to tones.warning
        AccountState.rejected -> "Needs attention" to tones.warning
        OtpPrompt.message != null -> "OTP waiting" to tones.danger
        Activity.offline -> "Offline" to tones.warning
        else -> "Signed in" to tones.success
    }
    Box(
        modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)
            .clip(MaterialTheme.shapes.extraLarge).background(tones.accentTone.subtle).heroCookie()
            .testTag("settings.user")
            .semantics(mergeDescendants = true) {
                contentDescription = listOfNotNull(name, user, number, semester?.let(::shortSemesterName), state).joinToString(", ")
            },
    ) {
        Row(Modifier.fillMaxWidth().padding(18.dp), horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Box {
                Avatar(
                    (name ?: user)?.firstOrNull()?.uppercase().orEmpty(),
                    78.dp,
                    Modifier.tappable({ menu = true }, "Profile photo", Role.Button).testTag("settings.avatar"),
                    shape = MaterialShapes.Cookie9Sided.toShape(),
                    style = MaterialTheme.typography.headlineMediumEmphasized,
                )
                CameraBadge(28.dp, Modifier.align(Alignment.BottomEnd).offset(4.dp, 2.dp))
                AvatarMenu(menu, { menu = false }, vtopPhoto)
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(name ?: user ?: "…", style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (name != null && user != null) {
                    Text(user, style = MaterialTheme.typography.labelLarge, color = tones.accentTone.onSubtle, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                // The registration number shares the semester's line: that one has the room.
                listOfNotNull(number, semester?.let(::shortSemesterName)).takeIf { it.isNotEmpty() }?.let {
                    Text(it.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                // Offline: just the icon.
                if (state == "Offline") {
                    Box(Modifier.padding(top = 6.dp).background(tone.subtle, MaterialTheme.shapes.small).padding(horizontal = 8.dp, vertical = 3.dp).testTag("settings.offline")) {
                        Icon(Lucide.CloudOff, null, Modifier.size(16.dp), tint = tone.base)
                    }
                } else {
                    StatusChip(state, tone, Modifier.padding(top = 6.dp))
                }
            }
        }
    }
}
