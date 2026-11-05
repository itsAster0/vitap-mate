package vitapmate.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import vitapmate.data.Downloads

/**
 * Asks where saved files go: a folder of the student's (the system folder picker) or
 * Downloads/vitapmate. Dismissing keeps [Downloads.DEFAULT_LABEL] (or, from Settings, what was set).
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun DownloadFolderDialog(onDone: () -> Unit, onDismiss: () -> Unit = onDone) {
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { tree: Uri? ->
        if (tree != null) choose(tree) else onDismiss()
        onDone()
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        // A dialog is its own window: its tags need their own resource ids.
        modifier = Modifier.semantics { testTagsAsResourceId = true }.testTag("downloads.dialog"),
        title = { Text("Download folder") },
        confirmButton = { TextButton({ picker.launch(null) }, Modifier.testTag("downloads.choose")) { Text("Choose folder") } },
        dismissButton = { TextButton({ choose(null); onDone() }, Modifier.testTag("downloads.default")) { Text("Use ${Downloads.DEFAULT_LABEL}") } },
    )
}

private fun choose(tree: Uri?) = Downloads.choose(tree) { failed -> showToast("Some files didn't move", "$failed saved file${if (failed == 1) "" else "s"} stayed in the old folder.") }

/** At the app's root: the first save waits here for the folder choice ([Downloads.prompt]). */
@Composable
fun DownloadFolderPrompt() {
    if (Downloads.prompt != null) DownloadFolderDialog(onDone = {}, onDismiss = { Downloads.choose(null) })
}
