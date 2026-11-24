package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import vitapmate.core.Core
import vitapmate.core.WrappedReport
import vitapmate.data.Activity
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.features.wrapped.wrappedFeature
import vitapmate.observability.Log
import vitapmate.ui.AppTheme
import vitapmate.ui.formatDay
import vitapmate.ui.heroCookie
import vitapmate.ui.tappable
import vitapmate.data.attempt

/**
 * Semester Wrapped at the top of More, once the selected semester's classes are over (none
 * before): a tall card until that semester's Wrapped is opened, then a one-line row.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun WrappedCard() {
    val nav = LocalNav.current
    // Read again after each VTOP run (a calendar that arrives while More is open), like the
    // Changes count; Settings.wrappedSeen is state, so opening Wrapped turns the card into the row.
    val last = Activity.recent.firstOrNull()?.flow
    val report by produceState<WrappedReport?>(null, SemesterChoice.selected, last) {
        value = attempt {
            coreFlow("more.wrapped", Core::cachedWrapped)
        }.getOrElse { e ->
            // A failed read (a Rust panic) means no card, never a crash; the flow logged its error.
            Log.warn("ui.more", "wrapped card read failed", fields = mapOf("error" to (e::class.simpleName ?: "Throwable")))
            null
        }
    }
    val r = report ?: return
    val tones = AppTheme.tones
    val open = { nav.push(Route.Screen(wrappedFeature.id)) }
    val outer = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)
    if (r.semesterId in Settings.wrappedSeen) {
        Row(
            outer.height(60.dp).clip(CircleShape).background(tones.accentTone.subtle).heroCookie()
                .tappable(open, "${r.semesterName} wrapped", Role.Button)
                .padding(start = 10.dp, end = 18.dp).testTag("more.wrapped_row"),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(40.dp).background(tones.accentTone.base, MaterialShapes.Cookie9Sided.toShape()))
            Text("${r.semesterName} wrapped", Modifier.weight(1f), style = MaterialTheme.typography.titleMediumEmphasized, color = MaterialTheme.colorScheme.onSurface, maxLines = 1)
            Icon(Lucide.ChevronRight, null, tint = tones.accentTone.onSubtle)
        }
        return
    }
    val dates = "${formatDay(r.firstDay, "d MMM")} – ${formatDay(r.lastDay, "d MMM")}"
    Box(
        outer.heightIn(min = 168.dp).clip(MaterialTheme.shapes.extraLarge).background(tones.accentTone.subtle)
            .tappable(open, "Your ${r.semesterName}, wrapped, $dates", Role.Button).testTag("more.wrapped_card"),
    ) {
        // The cluster on the right; the cookie runs off the card's edge.
        Box(Modifier.align(Alignment.TopEnd).offset(38.dp, 38.dp).size(170.dp).background(tones.accentTone.base, MaterialShapes.Cookie9Sided.toShape()))
        Box(Modifier.align(Alignment.TopEnd).offset((-90).dp, 16.dp).size(70.dp).background(tones.success.base, MaterialShapes.Sunny.toShape()))
        Box(Modifier.align(Alignment.TopEnd).offset((-30).dp, 122.dp).rotate(-16f).size(84.dp, 32.dp).background(tones.warning.base, CircleShape))
        // A column, so a long semester name grows the card instead of running into the dates.
        Column(Modifier.padding(start = 20.dp, top = 22.dp, bottom = 18.dp).widthIn(max = 190.dp).heightIn(min = 128.dp), verticalArrangement = Arrangement.SpaceBetween) {
            Text("Your ${r.semesterName}, wrapped", style = MaterialTheme.typography.headlineSmallEmphasized, color = MaterialTheme.colorScheme.onSurface)
            Text("$dates · tap to see", Modifier.padding(top = 12.dp), style = MaterialTheme.typography.labelLarge, color = tones.accentTone.onSubtle)
        }
    }
}
