package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import vitapmate.data.SemesterChoice
import vitapmate.ui.Space
import vitapmate.ui.shortSemesterName

/**
 * Once per install, after the first semester pick: Settings' own rows for the things worth turning
 * on at the start (In class, Live updates, Background sync, Calendar), each asking for its
 * permission as it does there. [onDone] marks it seen.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Onboarding(onDone: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val semester = SemesterChoice.semesters?.let { list -> list.items.firstOrNull { it.id == list.selected } }
    Box(Modifier.fillMaxSize().background(scheme.background).systemBarsPadding().testTag("onboarding.screen"), Alignment.TopCenter) {
        Column(Modifier.widthIn(max = 480.dp).fillMaxSize()) {
            Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
                Box(Modifier.padding(start = Space.xl, top = Space.xxl).size(140.dp, 104.dp)) {
                    Box(Modifier.offset(56.dp, 46.dp).size(52.dp).background(scheme.tertiaryContainer, MaterialShapes.Circle.toShape()))
                    Box(Modifier.offset(88.dp, 0.dp).size(38.dp).rotate(18f).background(scheme.secondaryContainer, MaterialShapes.Square.toShape()))
                    Box(Modifier.size(88.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                        Icon(Lucide.Sparkles, null, Modifier.size(36.dp), tint = scheme.onPrimaryContainer)
                    }
                }
                Spacer(Modifier.height(Space.lg))
                Text("Make it yours", Modifier.padding(horizontal = Space.xl), style = MaterialTheme.typography.displaySmallEmphasized)
                Spacer(Modifier.height(Space.sm))
                Text(
                    "${semester?.let(::shortSemesterName) ?: "Your semester"} is set. Turn on what helps; all of it stays in Settings.",
                    Modifier.padding(horizontal = Space.xl),
                    style = MaterialTheme.typography.bodyLarge,
                    color = scheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(Space.md))
                InsightsSetting()
                ClassModeSetting()
                LiveUpdatesSetting()
                BackgroundSyncSetting {}
                CalendarSync.Setting()
                Spacer(Modifier.height(Space.lg))
            }
            Button(
                onDone,
                modifier = Modifier.fillMaxWidth().padding(horizontal = Space.xl, vertical = Space.lg)
                    .heightIn(min = ButtonDefaults.MediumContainerHeight).testTag("onboarding.done"),
                shapes = ButtonDefaults.shapes(),
            ) {
                Text("Done", style = MaterialTheme.typography.titleSmall)
            }
        }
    }
}
