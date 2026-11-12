package vitapmate.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.FolderOpen
import com.composables.icons.lucide.Lucide
import androidx.compose.material3.Text
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
@OptIn(ExperimentalComposeUiApi::class, ExperimentalMaterial3ExpressiveApi::class)
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
        // The two choices as rows of one joined block; no buttons below them.
        text = {
            Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                Choice(Lucide.FolderOpen, "Choose folder", "downloads.choose", primary = true) { picker.launch(null) }
                Choice(Lucide.Download, "Use ${Downloads.DEFAULT_LABEL}", "downloads.default", primary = false) { choose(null); onDone() }
            }
        },
        confirmButton = {},
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Choice(icon: ImageVector, label: String, tag: String, primary: Boolean, onClick: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Row(
        Modifier.fillMaxWidth().heightIn(min = 64.dp).clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(scheme.surfaceContainerHighest)
            .tappable(onClick, label, Role.Button).padding(horizontal = 14.dp, vertical = 10.dp).testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.size(40.dp).background(if (primary) scheme.primary else scheme.secondaryContainer, RoundedCornerShape(if (primary) 14.dp else 20.dp)),
            Alignment.Center,
        ) {
            Icon(icon, null, Modifier.size(20.dp), tint = if (primary) scheme.onPrimary else scheme.onSecondaryContainer)
        }
        Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge, color = scheme.onSurface)
        Icon(Lucide.ChevronRight, null, tint = scheme.onSurfaceVariant)
    }
}

private fun choose(tree: Uri?) = Downloads.choose(tree) { failed -> showToast("Some files didn't move", "$failed saved file${if (failed == 1) "" else "s"} stayed in the old folder.") }

/** At the app's root: the first save waits here for the folder choice ([Downloads.prompt]). */
@Composable
fun DownloadFolderPrompt() {
    if (Downloads.prompt != null) DownloadFolderDialog(onDone = {}, onDismiss = { Downloads.choose(null) })
}
