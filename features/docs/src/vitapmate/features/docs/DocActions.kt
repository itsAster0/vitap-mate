package vitapmate.features.docs

import vitapmate.ui.OpenHaptic
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.webkit.MimeTypeMap
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.core.content.FileProvider
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.PencilLine
import com.composables.icons.lucide.Share2
import com.composables.icons.lucide.Sheet
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.Utensils
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import vitapmate.data.Doc
import vitapmate.data.DocKind
import vitapmate.data.Docs
import vitapmate.data.MessMenuFormatException
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space
import vitapmate.ui.showToast

/**
 * [doc]'s content URI in the download folder, shared as is; a plain file (Android 9 and older,
 * when the public Downloads wasn't writable) goes through the app's FileProvider instead.
 */
private fun uriOf(context: Context, doc: Doc): Uri = Docs.uriOf(doc).let { uri ->
    if (uri.scheme == "file") FileProvider.getUriForFile(context, "${context.packageName}.logs", java.io.File(uri.path!!)) else uri
}

private fun mimeOf(doc: Doc): String =
    MimeTypeMap.getSingleton().getMimeTypeFromExtension(doc.file.substringAfterLast('.', "").lowercase()) ?: "application/octet-stream"

/** Shares [doc] through the share sheet. */
internal fun share(context: Context, doc: Doc) {
    val uri = uriOf(context, doc)
    val send = Intent(Intent.ACTION_SEND).setType(mimeOf(doc)).putExtra(Intent.EXTRA_STREAM, uri).putExtra(Intent.EXTRA_SUBJECT, doc.name)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    send.clipData = ClipData.newRawUri(doc.name, uri)
    context.startActivity(Intent.createChooser(send, doc.name))
}

/** Opens [doc] in another app; a toast when none can. */
internal fun openWith(context: Context, doc: Doc) {
    val view = Intent(Intent.ACTION_VIEW).setDataAndType(uriOf(context, doc), mimeOf(doc)).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    try {
        context.startActivity(Intent.createChooser(view, doc.name))
    } catch (_: ActivityNotFoundException) {
        showToast("No app can open this", "Install one that reads this kind of file.")
    }
}

/** What a long press on a document offers, in a sheet; the dialogs it opens are drawn here too. */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
internal fun DocActions(doc: Doc, scope: CoroutineScope, onDismiss: () -> Unit) {
    val context = LocalContext.current
    // The sheet's own scope ends with it; [act] runs on the screen's [scope].
    val sheetScope = rememberCoroutineScope()
    var dialog by remember { mutableStateOf<String?>(null) }
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    fun act(name: String, block: suspend (String) -> Unit) {
        scope.launch {
            try {
                flow("docs.$name") { f ->
                    block(f)
                    Log.info(SRC, name, f, mapOf("kind" to doc.kind.name, "count" to "${Docs.docs.size}"))
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: MessMenuFormatException) {
                showToast("Not a mess menu", "${e.message} It stays a spreadsheet.")
            } catch (e: Throwable) {
                Log.warn(SRC, "$name failed: ${e::class.simpleName}")
                showToast("Something went wrong", "Please try again.")
            }
        }
    }
    // Closes the sheet first, then runs [then].
    fun close(then: () -> Unit) {
        sheetScope.launch { sheet.hide() }.invokeOnCompletion { then() }
    }

    if (dialog == null) {
        OpenHaptic()
        ModalBottomSheet(
            onDismiss,
            Modifier.semantics { testTagsAsResourceId = true }.testTag("docs.actions"),
            sheet,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            CompositionLocalProvider(LocalContentPadding provides PaddingValues()) {
                Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(bottom = Space.sm)) {
                    Text(
                        doc.name,
                        Modifier.padding(horizontal = Space.xl).padding(bottom = Space.sm),
                        style = MaterialTheme.typography.titleLargeEmphasized,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Action(Lucide.PencilLine, "Rename", "docs.action.rename") { close { dialog = "rename" } }
                    if (doc.kind == DocKind.Spreadsheet) {
                        Action(if (doc.asMessMenu) Lucide.Sheet else Lucide.Utensils, if (doc.asMessMenu) "Show as spreadsheet" else "Show as mess menu", "docs.action.menu") {
                            close {
                                onDismiss()
                                act("menu") { if (!doc.asMessMenu) loadMessMenu(doc); Docs.setMessMenu(doc.id, !doc.asMessMenu) }
                            }
                        }
                    }
                    Action(Lucide.Share2, "Share", "docs.action.share") { close { onDismiss(); act("share") { share(context, doc) } } }
                    Action(Lucide.ExternalLink, "Open with…", "docs.action.open_with") { close { onDismiss(); act("open_with") { openWith(context, doc) } } }
                    Action(Lucide.Trash2, "Delete", "docs.action.delete", AppTheme.tones.danger.base) { close { dialog = "delete" } }
                }
            }
        }
    }
    when (dialog) {
        "rename" -> {
            var name by remember { mutableStateOf(TextFieldValue(doc.name, TextRange(0, doc.name.length))) }
            val focus = remember { FocusRequester() }
            LaunchedEffect(Unit) { focus.requestFocus() }
            AlertDialog(
                onDismissRequest = onDismiss,
            modifier = Modifier.semantics { testTagsAsResourceId = true },
                title = { Text("Rename document") },
                text = { OutlinedTextField(name, { name = it }, Modifier.fillMaxWidth().focusRequester(focus).testTag("docs.rename.field"), singleLine = true) },
                confirmButton = {
                    TextButton({ onDismiss(); act("rename") { Docs.rename(doc.id, name.text) } }, Modifier.testTag("docs.rename.save"), enabled = name.text.isNotBlank()) { Text("Save") }
                },
                dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
            )
        }
        "delete" -> AlertDialog(
            onDismissRequest = onDismiss,
            modifier = Modifier.semantics { testTagsAsResourceId = true },
            title = { Text("Delete \"${doc.name}\"?") },
            confirmButton = { TextButton({ onDismiss(); act("delete") { Docs.remove(doc.id) } }, Modifier.testTag("docs.delete.confirm")) { Text("Delete") } },
            dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
        )
    }
}

@Composable
private fun Action(icon: ImageVector, label: String, tag: String, tint: Color = Color.Unspecified, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(label, color = if (tint == Color.Unspecified) MaterialTheme.colorScheme.onSurface else tint) },
        modifier = Modifier.testTag(tag).clickable(onClick = onClick).padding(horizontal = Space.sm),
        leadingContent = { Icon(icon, null, tint = if (tint == Color.Unspecified) MaterialTheme.colorScheme.onSurfaceVariant else tint) },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
    )
}
