package vitapmate.ui

import android.util.Base64
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.IdCard
import com.composables.icons.lucide.ImagePlus
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Trash2
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch
import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * The avatar's menu: Choose photo (the Photo Picker, no storage permission), Use VTOP photo when
 * [vtopPhoto] (base64) is given, and Remove photo once one is set. Flows `profile.photo.pick`,
 * `profile.photo.vtop`, `profile.photo.remove`; log src `ui.profile`.
 */
@Composable
fun AvatarMenu(expanded: Boolean, onDismiss: () -> Unit, vtopPhoto: String? = null) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        scope.launch {
            try {
                flow("profile.photo.pick") { id ->
                    if (uri == null) return@flow Log.info("ui.profile", "photo pick cancelled", id)
                    AvatarPhoto.pick(context, uri)
                    Log.info("ui.profile", "photo set", id)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Throwable) {
                showToast("Couldn't use that photo", "Try another image.", tone = ToastTone.Error)
            }
        }
    }
    DropdownMenu(expanded, onDismiss, Modifier.testTag("avatar.menu")) {
        DropdownMenuItem(
            text = { Text("Choose photo") },
            onClick = {
                onDismiss()
                picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
            },
            leadingIcon = { Icon(Lucide.ImagePlus, null) },
            modifier = Modifier.testTag("avatar.choose"),
        )
        if (!vtopPhoto.isNullOrEmpty()) {
            DropdownMenuItem(
                text = { Text("Use VTOP photo") },
                onClick = {
                    onDismiss()
                    scope.launch {
                        runCatching {
                            flow("profile.photo.vtop") { id ->
                                AvatarPhoto.use(context, Base64.decode(vtopPhoto, Base64.DEFAULT))
                                Log.info("ui.profile", "photo set from VTOP", id)
                            }
                        }.onFailure { if (it is CancellationException) throw it else showToast("Couldn't use the VTOP photo", "Choose one instead.", tone = ToastTone.Error) }
                    }
                },
                leadingIcon = { Icon(Lucide.IdCard, null) },
                modifier = Modifier.testTag("avatar.vtop"),
            )
        }
        if (Settings.avatarPath != null) {
            DropdownMenuItem(
                text = { Text("Remove photo") },
                onClick = {
                    onDismiss()
                    scope.launch { flow("profile.photo.remove") { id -> AvatarPhoto.remove().also { Log.info("ui.profile", "photo removed", id) } } }
                },
                leadingIcon = { Icon(Lucide.Trash2, null) },
                modifier = Modifier.testTag("avatar.remove"),
            )
        }
    }
}
