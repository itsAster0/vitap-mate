package vitapmate.features.login

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.OtpPrompt
import vitapmate.data.SemesterChoice
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.ProblemBanner
import vitapmate.ui.SemesterList
import vitapmate.ui.Space

/**
 * After sign-in, and whenever no semester is picked: VTOP's semesters (newest first, nothing
 * pre-picked, as the Flutter app's onboarding) and "Continue" once the student taps one. The
 * list is the cached one at once, then fresh from VTOP; a failure shows with "Try again". The
 * app moves on when [SemesterChoice.selected] is set. Flow `semesters.choose` (open, pick); log
 * src `ui.semesters`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ChooseSemester() {
    val scheme = MaterialTheme.colorScheme
    val scope = rememberCoroutineScope()
    var chosen by rememberSaveable { mutableStateOf<String?>(null) }
    val items = SemesterChoice.semesters?.items.orEmpty()
    val problem = SemesterChoice.problem
    LaunchedEffect(Unit) {
        flow("semesters.choose") { f ->
            SemesterChoice.readCache(f)
            Log.info("ui.semesters", "choose open", f, mapOf("cached" to "${SemesterChoice.semesters?.items?.size ?: 0}"))
        }
        SemesterChoice.refresh("choose")
    }
    // The OTP a refresh asked for was entered: load the list again.
    val otpAtStart = rememberSaveable { OtpPrompt.generation }
    LaunchedEffect(OtpPrompt.generation) {
        if (OtpPrompt.generation != otpAtStart) SemesterChoice.refresh("otp")
    }

    Box(Modifier.fillMaxSize().background(scheme.background).systemBarsPadding().testTag("semester.choose.screen"), Alignment.TopCenter) {
        Column(Modifier.widthIn(max = 480.dp).fillMaxSize().padding(horizontal = Space.xl), horizontalAlignment = Alignment.CenterHorizontally) {
            Spacer(Modifier.height(Space.xxl))
            Box(Modifier.size(72.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Icon(Lucide.CalendarRange, null, Modifier.size(32.dp), tint = scheme.onPrimaryContainer)
            }
            Spacer(Modifier.height(Space.lg))
            Text("Choose your semester", style = MaterialTheme.typography.headlineMediumEmphasized, textAlign = TextAlign.Center)
            Spacer(Modifier.height(Space.sm))
            Text(
                "The app shows this semester's timetable, attendance and marks. You can change it later in Settings → Account → Semester.",
                style = MaterialTheme.typography.bodyLarge,
                color = scheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(Space.lg))
            AnimatedVisibility(problem != null) {
                problem?.let {
                    val retry: () -> Unit = { scope.launch { SemesterChoice.refresh("retry") } }
                    ProblemBanner(it, onDismiss = {}, Modifier.padding(bottom = Space.sm).testTag("semester.choose.problem"), onRetry = retry)
                }
            }
            Box(Modifier.weight(1f).fillMaxWidth(), Alignment.Center) {
                when {
                    items.isNotEmpty() -> SemesterList(
                        items,
                        chosen,
                        { chosen = it },
                        Modifier.fillMaxSize().testTag("semester.choose.list"),
                        PaddingValues(bottom = Space.lg),
                    )
                    SemesterChoice.loading || problem == null -> LoadingIndicator(Modifier.size(48.dp).testTag("semester.choose.loading"))
                }
            }
            Button(
                {
                    val id = chosen ?: return@Button
                    scope.launch {
                        flow("semesters.choose") { f -> Log.info("ui.semesters", "choose pick", f, mapOf("semester" to id)) }
                        SemesterChoice.pick(id)
                    }
                },
                modifier = Modifier.fillMaxWidth().heightIn(min = ButtonDefaults.MediumContainerHeight).testTag("semester.choose.continue"),
                enabled = chosen != null && items.any { it.id == chosen },
                shapes = ButtonDefaults.shapes(),
            ) {
                Text("Continue", style = MaterialTheme.typography.titleSmall)
            }
            Spacer(Modifier.height(Space.lg))
        }
    }
}
